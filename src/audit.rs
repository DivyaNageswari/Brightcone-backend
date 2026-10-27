use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait};

use crate::{
    models::{_entities::audit_logs::ActiveModel, error::ModelError, users},
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

pub async fn record<C: ConnectionTrait>(
    db: &C,
    actor: Option<&users::Model>,
    meta: &RequestMeta,
    entry: Entry<'_>,
) -> Result<()> {
    let now = chrono::Local::now().fixed_offset();
    ActiveModel {
        actor_user_id: ActiveValue::set(actor.map(|u| u.id)),
        actor_email: ActiveValue::set(actor.map(|u| u.email.clone())),
        action: ActiveValue::set(entry.action.to_string()),
        target_type: ActiveValue::set(entry.target_type.map(|s| s.to_string())),
        target_id: ActiveValue::set(entry.target_id),
        summary: ActiveValue::set(entry.summary),
        ip: ActiveValue::set(meta.ip.clone()),
        request_id: ActiveValue::set(meta.request_id.clone()),
        created_at: ActiveValue::set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ModelError::from)?;
    Ok(())
}
