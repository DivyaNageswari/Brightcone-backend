use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    audit::{self, Entry},
    authz::AuthUser,
    models::_entities::{
        hr_rule_events::{self, Entity as HrRuleEvents},
        hr_rules::{self, Entity as HrRules},
    },
    request_meta::RequestMeta,
    workers::rule_dispatch::{RuleDispatchArgs, RuleDispatchWorker},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Rule {
    pub rule_key: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RuleUpdate {
    pub enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RuleMetrics {
    pub pending: i64,
    pub complete: i64,
    pub failed: i64,
}

#[utoipa::path(
    get,
    path = "/api/rules",
    security(("bearer" = [])),
    responses((status = 200, body = Vec<Rule>))
)]
#[debug_handler]
pub async fn list(
    auth: AuthUser,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    auth.require("rules:manage")?;
    let rules = HrRules::find()
        .order_by_asc(hr_rules::Column::RuleKey)
        .all(&ctx.db)
        .await?;

    let items: Vec<Rule> = rules
        .into_iter()
        .map(|r| Rule {
            rule_key: r.rule_key,
            enabled: r.enabled,
        })
        .collect();

    format::json(items)
}

#[utoipa::path(
    put,
    path = "/api/rules/{rule_key}",
    security(("bearer" = [])),
    params(("rule_key" = String, Path, description = "Rule key identifier")),
    request_body = RuleUpdate,
    responses((status = 200, body = Rule), (status = 404, description = "Rule not found"))
)]
#[debug_handler]
pub async fn update(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    meta: RequestMeta,
    Path(key): Path<String>,
    Json(body): Json<RuleUpdate>,
) -> Result<Response> {
    auth.require("rules:manage")?;
    let found = HrRules::find_by_id(&key).one(&ctx.db).await?;
    let Some(rule) = found else {
        return Err(Error::NotFound);
    };

    let mut active = rule.into_active_model();
    active.enabled = ActiveValue::set(body.enabled);
    active.updated_by = ActiveValue::set(Some(auth.user.id));
    active.updated_at = ActiveValue::set(chrono::Local::now().fixed_offset());
    let updated = active.update(&ctx.db).await?;

    audit::record(
        &ctx.db,
        Some(&auth.user),
        &meta,
        Entry::new("rule.updated")
            .target("rule", &key)
            .summary(format!("enabled={}", body.enabled)),
    )
    .await?;

    format::json(Rule {
        rule_key: updated.rule_key,
        enabled: updated.enabled,
    })
}

#[utoipa::path(
    get,
    path = "/api/rules/metrics",
    security(("bearer" = [])),
    responses((status = 200, body = RuleMetrics))
)]
#[debug_handler]
pub async fn metrics(
    auth: AuthUser,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    auth.require("rules:manage")?;
    let pending = HrRuleEvents::find()
        .filter(hr_rule_events::Column::Status.eq("pending"))
        .count(&ctx.db)
        .await? as i64;
    let complete = HrRuleEvents::find()
        .filter(hr_rule_events::Column::Status.eq("complete"))
        .count(&ctx.db)
        .await? as i64;
    let failed = HrRuleEvents::find()
        .filter(hr_rule_events::Column::Status.eq("failed"))
        .count(&ctx.db)
        .await? as i64;

    format::json(RuleMetrics {
        pending,
        complete,
        failed,
    })
}

#[utoipa::path(
    post,
    path = "/api/rules/events/{id}/retry",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Event ID")),
    responses((status = 200, description = "Event enqueued for retry"), (status = 404, description = "Event not found"))
)]
#[debug_handler]
pub async fn retry(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    auth.require("rules:manage")?;
    let events = HrRuleEvents::find()
        .filter(hr_rule_events::Column::EventId.eq(id))
        .all(&ctx.db)
        .await?;

    if events.is_empty() {
        return Err(Error::NotFound);
    }

    for ev in events {
        let mut active = ev.into_active_model();
        active.status = ActiveValue::set("pending".to_string());
        active.last_error = ActiveValue::set(None);
        active.update(&ctx.db).await?;
    }

    RuleDispatchWorker::perform_later(&ctx, RuleDispatchArgs { event_id: id }).await?;
    format::json(serde_json::json!({ "status": "retrying" }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/rules")
        .add("/", get(list))
        .add("/metrics", get(metrics))
        .add("/events/{id}/retry", post(retry))
        .add("/{rule_key}", put(update))
}