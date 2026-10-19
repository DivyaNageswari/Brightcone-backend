//! Audit log service. Passwords, tokens and full salary figures never enter summaries.
use diesel::{
    sql_query,
    sql_types::{BigInt, Nullable, Text},
};
use diesel_async::RunQueryDsl;
use loco_rs::prelude::*;

use crate::{
    database::Database,
    models::{error::ModelError, users},
    request_meta::RequestMeta,
};

pub struct Entry<'a> {
    pub action: &'a str,
    pub target_type: Option<&'a str>,
    pub target_id: Option<String>,
    pub summary: Option<String>,
}

impl<'a> Entry<'a> {
    #[must_use]
    pub const fn new(action: &'a str) -> Self {
        Self {
            action,
            target_type: None,
            target_id: None,
            summary: None,
        }
    }

    #[must_use]
    pub fn target(mut self, kind: &'a str, id: impl ToString) -> Self {
        self.target_type = Some(kind);
        self.target_id = Some(id.to_string());
        self
    }

    #[must_use]
    pub fn summary(mut self, text: impl Into<String>) -> Self {
        self.summary = Some(text.into());
        self
    }
}

pub async fn record(
    db: &Database,
    actor: Option<&users::Model>,
    meta: &RequestMeta,
    entry: Entry<'_>,
) -> Result<()> {
    let mut conn = db.get().await?;
    sql_query(
        "INSERT INTO audit_logs \
         (actor_user_id, actor_email, action, target_type, target_id, summary, ip, request_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind::<Nullable<BigInt>, _>(actor.map(|user| user.id))
    .bind::<Nullable<Text>, _>(actor.map(|user| user.email.clone()))
    .bind::<Text, _>(entry.action)
    .bind::<Nullable<Text>, _>(entry.target_type)
    .bind::<Nullable<Text>, _>(entry.target_id)
    .bind::<Nullable<Text>, _>(entry.summary)
    .bind::<Nullable<Text>, _>(meta.ip.clone())
    .bind::<Nullable<Text>, _>(meta.request_id.clone())
    .execute(&mut *conn)
    .await
    .map_err(ModelError::from)?;
    Ok(())
}
