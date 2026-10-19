use chrono::{Duration, Utc};
use diesel::{
    prelude::QueryableByName,
    sql_query,
    sql_types::{BigInt, Nullable, Text, Timestamptz},
    OptionalExtension,
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use loco_rs::hash;

use crate::{
    database::Database,
    models::{
        error::{ModelError, ModelResult},
        refresh_tokens::hash_token,
    },
};

#[derive(QueryableByName)]
struct ResetRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    used_at: Option<chrono::DateTime<Utc>>,
    #[diesel(sql_type = Timestamptz)]
    expires_at: chrono::DateTime<Utc>,
}

pub struct Model;

impl Model {
    pub async fn issue(db: &Database, user_id: i64, minutes: i64) -> ModelResult<String> {
        let secret = hash::random_string(48);
        let token_hash = hash_token(&secret);
        let expires_at = Utc::now() + Duration::minutes(minutes);
        let mut conn = db.get().await.map_err(db_error)?;
        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
            sql_query(
                "UPDATE password_resets SET used_at = now() WHERE user_id = $1 AND used_at IS NULL",
            )
            .bind::<BigInt, _>(user_id)
            .execute(conn)
            .await?;
            sql_query(
                "INSERT INTO password_resets (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
            )
            .bind::<BigInt, _>(user_id)
            .bind::<Text, _>(token_hash.clone())
            .bind::<Timestamptz, _>(expires_at)
            .execute(conn)
            .await?;
            Ok(())
        })
        .await
        .map_err(ModelError::from)?;
        Ok(secret)
    }

    pub async fn consume(db: &Database, secret: &str) -> ModelResult<Option<i64>> {
        let token_hash = hash_token(secret);
        let mut conn = db.get().await.map_err(db_error)?;
        conn.transaction::<_, diesel::result::Error, _>(async |conn| {
            let row = sql_query(
                "SELECT user_id, used_at, expires_at FROM password_resets \
                 WHERE token_hash = $1 FOR UPDATE",
            )
            .bind::<Text, _>(token_hash.clone())
            .get_result::<ResetRow>(conn)
            .await
            .optional()?;
            let Some(row) = row else { return Ok(None) };
            if row.used_at.is_some() || row.expires_at < Utc::now() {
                return Ok(None);
            }
            sql_query("UPDATE password_resets SET used_at = now() WHERE token_hash = $1")
                .bind::<Text, _>(token_hash)
                .execute(conn)
                .await?;
            Ok(Some(row.user_id))
        })
        .await
        .map_err(ModelError::from)
    }
}

fn db_error(error: loco_rs::Error) -> ModelError {
    ModelError::Database(error.to_string())
}
