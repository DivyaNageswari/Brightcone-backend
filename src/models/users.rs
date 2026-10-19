use chrono::{DateTime, Utc};
use diesel::{
    prelude::QueryableByName,
    sql_query,
    sql_types::{BigInt, Bool, Integer, Nullable, Text, Timestamptz, Uuid as SqlUuid},
    OptionalExtension,
};
use diesel_async::RunQueryDsl;
use loco_rs::{auth::jwt, hash};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use uuid::Uuid;
use validator::Validate;

use crate::{
    authz::Role,
    database::Database,
    models::error::{ModelError, ModelResult},
};

pub const MIN_PASSWORD_LEN: usize = 12;
pub(crate) const USER_SELECT: &str = "SELECT id, created_at, updated_at, pid, email::text AS email, password, api_key, name, role, status, failed_logins, locked_at, last_login_at, must_change_password, password_changed_at FROM users";
const COMMON_PASSWORDS: &[&str] = &[
    "password1234",
    "password12345",
    "123456789012",
    "qwertyuiop12",
    "welcome12345",
    "letmein12345",
    "administrator",
    "iloveyou1234",
    "changeme1234",
    "brightcone123",
];

#[derive(Debug)]
pub struct NewUser {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Role,
    pub must_change_password: bool,
}

#[derive(Debug, Validate)]
pub struct Validator {
    #[validate(length(min = 2, max = 200, message = "Name must be 2 to 200 characters long."))]
    pub name: String,
    #[validate(email(message = "invalid email"), length(max = 254))]
    pub email: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, QueryableByName)]
pub struct Model {
    #[diesel(sql_type = BigInt)]
    pub id: i64,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = SqlUuid)]
    pub pid: Uuid,
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Text)]
    pub password: String,
    #[diesel(sql_type = Text)]
    pub api_key: String,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub role: String,
    #[diesel(sql_type = Text)]
    pub status: String,
    #[diesel(sql_type = Integer)]
    pub failed_logins: i32,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub locked_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub last_login_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Bool)]
    pub must_change_password: bool,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub password_changed_at: Option<DateTime<Utc>>,
}

impl Model {
    pub fn validator(&self) -> Validator {
        Validator {
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }

    pub async fn find_by_email(db: &Database, email: &str) -> ModelResult<Self> {
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(format!("{USER_SELECT} WHERE lower(email) = $1 LIMIT 1"))
            .bind::<Text, _>(normalize_email(email))
            .get_result(&mut *conn)
            .await
            .map_err(ModelError::from)
    }

    pub async fn find_by_pid(db: &Database, pid: &str) -> ModelResult<Self> {
        let pid = Uuid::parse_str(pid).map_err(|error| ModelError::Other(error.to_string()))?;
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(format!("{USER_SELECT} WHERE pid = $1 LIMIT 1"))
            .bind::<SqlUuid, _>(pid)
            .get_result(&mut *conn)
            .await
            .map_err(ModelError::from)
    }

    pub async fn find_by_api_key(db: &Database, api_key: &str) -> ModelResult<Self> {
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(format!("{USER_SELECT} WHERE api_key = $1 LIMIT 1"))
            .bind::<Text, _>(api_key)
            .get_result(&mut *conn)
            .await
            .map_err(ModelError::from)
    }

    pub async fn find_by_id(db: &Database, id: i64) -> ModelResult<Self> {
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(format!("{USER_SELECT} WHERE id = $1 LIMIT 1"))
            .bind::<BigInt, _>(id)
            .get_result(&mut *conn)
            .await
            .map_err(ModelError::from)
    }

    pub async fn find_optional_by_id(db: &Database, id: i64) -> ModelResult<Option<Self>> {
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(format!("{USER_SELECT} WHERE id = $1 LIMIT 1"))
            .bind::<BigInt, _>(id)
            .get_result(&mut *conn)
            .await
            .optional()
            .map_err(ModelError::from)
    }

    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        hash::verify_password(password, &self.password)
    }

    #[must_use]
    pub fn role(&self) -> Option<Role> {
        Role::parse(&self.role)
    }

    pub async fn create_with_password(db: &Database, params: &NewUser) -> ModelResult<Self> {
        Validator {
            name: params.name.trim().to_owned(),
            email: params.email.clone(),
        }
        .validate()
        .map_err(|error| ModelError::Validation(error.to_string()))?;
        check_password_strength(&params.password)
            .map_err(|error| ModelError::Validation(error.into()))?;
        let email = normalize_email(&params.email);
        let password = hash::hash_password(&params.password)
            .map_err(|error| ModelError::Other(error.to_string()))?;
        let pid = Uuid::new_v4();
        let api_key = format!("lo-{}", Uuid::new_v4());
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(
            "INSERT INTO users (pid, email, password, api_key, name, role, must_change_password) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        )
        .bind::<SqlUuid, _>(pid)
        .bind::<Text, _>(email)
        .bind::<Text, _>(password)
        .bind::<Text, _>(api_key)
        .bind::<Text, _>(params.name.trim())
        .bind::<Text, _>(params.role.as_str())
        .bind::<Bool, _>(params.must_change_password)
        .get_result(&mut *conn)
        .await
        .map_err(ModelError::from)
    }

    pub fn generate_jwt(&self, secret: &str, expiration: u64) -> ModelResult<String> {
        jwt::JWT::new(secret)
            .generate_token(expiration, self.pid.to_string(), Map::new())
            .map_err(|error| ModelError::Other(error.to_string()))
    }

    pub async fn register_failed_login(
        db: &Database,
        id: i64,
        threshold: i32,
    ) -> ModelResult<Self> {
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(
            "UPDATE users SET failed_logins = failed_logins + 1, \
             status = CASE WHEN failed_logins + 1 >= $2 AND status = 'active' THEN 'locked' ELSE status END, \
             locked_at = CASE WHEN failed_logins + 1 >= $2 AND status = 'active' THEN now() ELSE locked_at END, \
             updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind::<BigInt, _>(id)
        .bind::<Integer, _>(threshold)
        .get_result(&mut *conn)
        .await
        .map_err(ModelError::from)
    }

    pub async fn record_login_success(&self, db: &Database) -> ModelResult<Self> {
        update_user(
            db,
            self.id,
            "failed_logins = 0, last_login_at = now(), updated_at = now()",
        )
        .await
    }

    pub async fn set_password(
        &self,
        db: &Database,
        password: &str,
        must_change: bool,
    ) -> ModelResult<Self> {
        check_password_strength(password).map_err(|error| ModelError::Validation(error.into()))?;
        let password =
            hash::hash_password(password).map_err(|error| ModelError::Other(error.to_string()))?;
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(
            "UPDATE users SET password = $2, must_change_password = $3, password_changed_at = now(), \
             updated_at = now() \
             WHERE id = $1 RETURNING *",
        )
        .bind::<BigInt, _>(self.id)
        .bind::<Text, _>(password)
        .bind::<Bool, _>(must_change)
        .get_result(&mut *conn)
        .await
        .map_err(ModelError::from)
    }

    pub async fn set_status(&self, db: &Database, status: &str) -> ModelResult<Self> {
        if !["active", "locked", "disabled"].contains(&status) {
            return Err(ModelError::Validation("Invalid account status".into()));
        }
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query(
            "UPDATE users SET status = $2, failed_logins = 0, \
             locked_at = CASE WHEN $2 = 'locked' THEN now() ELSE NULL END, updated_at = now() \
             WHERE id = $1 RETURNING *",
        )
        .bind::<BigInt, _>(self.id)
        .bind::<Text, _>(status)
        .get_result(&mut *conn)
        .await
        .map_err(ModelError::from)
    }

    pub async fn set_role(&self, db: &Database, role: &str) -> ModelResult<Self> {
        if Role::parse(role).is_none() {
            return Err(ModelError::Validation("Invalid account role".into()));
        }
        let mut conn = db.get().await.map_err(database_error)?;
        sql_query("UPDATE users SET role = $2, updated_at = now() WHERE id = $1 RETURNING *")
            .bind::<BigInt, _>(self.id)
            .bind::<Text, _>(role)
            .get_result(&mut *conn)
            .await
            .map_err(ModelError::from)
    }
}

async fn update_user(db: &Database, id: i64, assignments: &str) -> ModelResult<Model> {
    let mut conn = db.get().await.map_err(database_error)?;
    sql_query(format!(
        "UPDATE users SET {assignments} WHERE id = $1 RETURNING *"
    ))
    .bind::<BigInt, _>(id)
    .get_result(&mut *conn)
    .await
    .map_err(ModelError::from)
}

fn database_error(error: loco_rs::Error) -> ModelError {
    ModelError::Database(error.to_string())
}

#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn check_password_strength(password: &str) -> Result<(), &'static str> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err("Password must be at least 12 characters long.");
    }
    let lower = password.to_lowercase();
    if COMMON_PASSWORDS.contains(&lower.as_str()) {
        return Err("That password is too common. Choose another.");
    }
    let first = password.chars().next();
    if password.chars().all(|character| Some(character) == first) {
        return Err("Password must not be a single repeated character.");
    }
    Ok(())
}
