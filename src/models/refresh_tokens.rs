use chrono::{DateTime, Duration, Utc};
use diesel::{
    prelude::QueryableByName,
    sql_query,
    sql_types::{BigInt, Bool, Nullable, Text, Timestamptz, Uuid as SqlUuid},
    OptionalExtension,
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use loco_rs::hash;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    database::Database,
    models::error::{ModelError, ModelResult},
    settings::AppSettings,
};

#[derive(Debug)]
pub struct Issued {
    pub secret: String,
    pub expires_at: DateTime<Utc>,
    pub persistent: bool,
    pub user_id: i64,
}

#[derive(Debug)]
pub enum Rotation {
    Rotated(Issued),
    Reused { user_id: i64 },
    Invalid,
}

#[derive(QueryableByName)]
struct TokenRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = SqlUuid)]
    family_id: Uuid,
    #[diesel(sql_type = Bool)]
    persistent: bool,
    #[diesel(sql_type = Timestamptz)]
    expires_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    used_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    revoked_at: Option<DateTime<Utc>>,
}

pub struct Model;

#[must_use]
pub fn hash_token(secret: &str) -> String {
    format!("{:x}", Sha256::digest(secret.as_bytes()))
}

impl Model {
    pub async fn issue(
        db: &Database,
        settings: &AppSettings,
        user_id: i64,
        family: Option<Uuid>,
        persistent: bool,
        ip: Option<String>,
    ) -> ModelResult<Issued> {
        let secret = hash::random_string(64);
        let token_hash = hash_token(&secret);
        let family_id = family.unwrap_or_else(Uuid::new_v4);
        let expires_at = expiry(settings, persistent);
        let mut conn = db.get().await.map_err(db_error)?;
        sql_query(
            "INSERT INTO refresh_tokens (user_id, token_hash, family_id, persistent, expires_at, created_ip) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind::<BigInt, _>(user_id)
        .bind::<Text, _>(token_hash)
        .bind::<SqlUuid, _>(family_id)
        .bind::<Bool, _>(persistent)
        .bind::<Timestamptz, _>(expires_at)
        .bind::<Nullable<Text>, _>(ip)
        .execute(&mut *conn)
        .await
        .map_err(ModelError::from)?;
        Ok(Issued {
            secret,
            expires_at,
            persistent,
            user_id,
        })
    }

    pub async fn rotate(
        db: &Database,
        settings: &AppSettings,
        presented: &str,
        ip: Option<String>,
    ) -> ModelResult<Rotation> {
        let presented_hash = hash_token(presented);
        let next_secret = hash::random_string(64);
        let next_hash = hash_token(&next_secret);
        let ip = ip.clone();
        let mut conn = db.get().await.map_err(db_error)?;
        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
            let found = sql_query(
                "SELECT user_id, family_id, persistent, expires_at, used_at, revoked_at \
                 FROM refresh_tokens WHERE token_hash = $1 FOR UPDATE",
            )
            .bind::<Text, _>(presented_hash.clone())
            .get_result::<TokenRow>(conn)
            .await
            .optional()?;
            let Some(row) = found else { return Ok(Rotation::Invalid) };

            if row.used_at.is_some() || row.revoked_at.is_some() {
                sql_query("UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL")
                    .bind::<SqlUuid, _>(row.family_id)
                    .execute(conn)
                    .await?;
                return Ok(Rotation::Reused { user_id: row.user_id });
            }
            if row.expires_at < Utc::now() {
                return Ok(Rotation::Invalid);
            }

            sql_query("UPDATE refresh_tokens SET used_at = now() WHERE token_hash = $1")
                .bind::<Text, _>(presented_hash)
                .execute(conn)
                .await?;
            let expires_at = expiry(settings, row.persistent);
            sql_query(
                "INSERT INTO refresh_tokens (user_id, token_hash, family_id, persistent, expires_at, created_ip) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind::<BigInt, _>(row.user_id)
            .bind::<Text, _>(next_hash)
            .bind::<SqlUuid, _>(row.family_id)
            .bind::<Bool, _>(row.persistent)
            .bind::<Timestamptz, _>(expires_at)
            .bind::<Nullable<Text>, _>(ip)
            .execute(conn)
            .await?;
            Ok(Rotation::Rotated(Issued {
                secret: next_secret,
                expires_at,
                persistent: row.persistent,
                user_id: row.user_id,
            }))
        })
        .await
        .map_err(ModelError::from)
    }

    pub async fn revoke_by_secret(db: &Database, secret: &str) -> ModelResult<Option<i64>> {
        let mut conn = db.get().await.map_err(db_error)?;
        let row = sql_query(
            "SELECT user_id, family_id, persistent, expires_at, used_at, revoked_at \
             FROM refresh_tokens WHERE token_hash = $1 LIMIT 1",
        )
        .bind::<Text, _>(hash_token(secret))
        .get_result::<TokenRow>(&mut *conn)
        .await
        .optional()
        .map_err(ModelError::from)?;
        if let Some(row) = row {
            Self::revoke_family(db, row.family_id).await?;
            Ok(Some(row.user_id))
        } else {
            Ok(None)
        }
    }

    pub async fn revoke_family(db: &Database, family: Uuid) -> ModelResult<()> {
        update_tokens(db, family).await
    }

    pub async fn revoke_all_for_user(db: &Database, user_id: i64) -> ModelResult<()> {
        let mut conn = db.get().await.map_err(db_error)?;
        sql_query("UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
            .bind::<BigInt, _>(user_id)
            .execute(&mut *conn)
            .await
            .map_err(ModelError::from)?;
        Ok(())
    }

    pub async fn delete_expired(db: &Database) -> ModelResult<u64> {
        let cutoff = Utc::now() - Duration::days(1);
        let mut conn = db.get().await.map_err(db_error)?;
        let count = sql_query("DELETE FROM refresh_tokens WHERE expires_at < $1")
            .bind::<Timestamptz, _>(cutoff)
            .execute(&mut *conn)
            .await
            .map_err(ModelError::from)?;
        Ok(u64::try_from(count).unwrap_or(u64::MAX))
    }
}

async fn update_tokens(db: &Database, family: Uuid) -> ModelResult<()> {
    let mut conn = db.get().await.map_err(db_error)?;
    sql_query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind::<SqlUuid, _>(family)
    .execute(&mut *conn)
    .await
    .map_err(ModelError::from)?;
    Ok(())
}

fn expiry(settings: &AppSettings, persistent: bool) -> DateTime<Utc> {
    let ttl = if persistent {
        Duration::days(settings.refresh_ttl_days)
    } else {
        Duration::hours(settings.session_ttl_hours)
    };
    Utc::now() + ttl
}

fn db_error(error: loco_rs::Error) -> ModelError {
    ModelError::Database(error.to_string())
}
