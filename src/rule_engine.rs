use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    models::_entities::{
        hr_notifications::{self, Entity as HrNotifications},
        hr_rule_events::{self, Entity as HrRuleEvents},
        hr_rules::{self, Entity as HrRules},
        users::{self, Entity as Users},
    },
    workers::rule_dispatch::{RuleDispatchArgs, RuleDispatchWorker},
};

const RULES: [&str; 9] = [
    "employee.created",
    "employee.updated",
    "employee.archived",
    "leave.submitted",
    "leave.decided",
    "expense.submitted",
    "expense.decided",
    "candidate.stage_changed",
    "payroll.completed",
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleEventPayload {
    pub record_id: Uuid,
    pub record_type: String,
    pub actor_user_id: i64,
}

pub fn event_key(
    record_type: &str,
    action: &str,
    before: Option<&serde_json::Value>,
    after: &serde_json::Value,
) -> Option<&'static str> {
    match (record_type, action) {
        ("employee", "created") => Some("employee.created"),
        ("employee", "updated") => Some("employee.updated"),
        ("employee", "archived") => Some("employee.archived"),
        ("leave_request", "created") if status(after) == Some("pending") => Some("leave.submitted"),
        ("leave_request", "updated") if transitioned(before, after, &["approved", "rejected"]) => {
            Some("leave.decided")
        }
        ("expense_claim", "created") if status(after) == Some("pending") => {
            Some("expense.submitted")
        }
        ("expense_claim", "updated") if transitioned(before, after, &["approved", "rejected"]) => {
            Some("expense.decided")
        }
        ("candidate", "updated")
            if before.and_then(|value| value.get("stage")) != after.get("stage")
                && after.get("stage").is_some() =>
        {
            Some("candidate.stage_changed")
        }
        ("payroll_run", "updated")
            if status(after) == Some("completed")
                && before.and_then(status) != Some("completed") =>
        {
            Some("payroll.completed")
        }
        _ => None,
    }
}

fn status(value: &serde_json::Value) -> Option<&str> {
    value.get("status").and_then(serde_json::Value::as_str)
}

fn transitioned(
    before: Option<&serde_json::Value>,
    after: &serde_json::Value,
    targets: &[&str],
) -> bool {
    let current = status(after);
    current.is_some_and(|value| targets.contains(&value)) && before.and_then(status) != current
}

pub async fn record_event<C: ConnectionTrait>(
    db: &C,
    key: &str,
    payload: &RuleEventPayload,
) -> Result<Option<Uuid>> {
    if !RULES.contains(&key) {
        return Ok(None);
    }

    let is_enabled = HrRules::find_by_id(key)
        .filter(hr_rules::Column::Enabled.eq(true))
        .one(db)
        .await?
        .is_some();

    if !is_enabled {
        return Ok(None);
    }

    let event_id = Uuid::new_v4();
    let json = serde_json::to_value(payload).unwrap_or_default();
    let now = chrono::Local::now().fixed_offset();

    let event = hr_rule_events::ActiveModel {
        event_id: ActiveValue::set(event_id),
        action_key: ActiveValue::set(key.to_string()),
        status: ActiveValue::set("pending".to_string()),
        attempts: ActiveValue::set(0),
        payload: ActiveValue::set(json),
        last_error: ActiveValue::set(None),
        created_at: ActiveValue::set(now),
        completed_at: ActiveValue::set(None),
    };
    event.insert(db).await?;
    Ok(Some(event_id))
}

pub async fn dispatch(ctx: &AppContext, event_id: Option<Uuid>) {
    let Some(event_id) = event_id else { return };
    if let Err(error) = RuleDispatchWorker::perform_later(ctx, RuleDispatchArgs { event_id }).await
    {
        tracing::error!(%event_id, error = %error, "rule event remains in the durable outbox");
    }
}

pub async fn enqueue_pending(ctx: &AppContext) -> Result<()> {
    let pending = HrRuleEvents::find()
        .filter(hr_rule_events::Column::Status.eq("pending"))
        .order_by_asc(hr_rule_events::Column::CreatedAt)
        .limit(500)
        .all(&ctx.db)
        .await?;

    if pending.is_empty() {
        return Ok(());
    }

    let args = pending
        .into_iter()
        .map(|row| RuleDispatchArgs {
            event_id: row.event_id,
        })
        .collect();
    RuleDispatchWorker::perform_all_later(ctx, args).await?;
    Ok(())
}

pub async fn process_event(ctx: &AppContext, event_id: Uuid) -> Result<()> {
    let events = HrRuleEvents::find()
        .filter(hr_rule_events::Column::EventId.eq(event_id))
        .all(&ctx.db)
        .await?;

    if events.is_empty() {
        return Ok(());
    }

    let now = chrono::Local::now().fixed_offset();

    for ev in events {
        if ev.status != "pending" {
            continue;
        }

        let new_attempts = ev.attempts + 1;
        if new_attempts > 5 {
            let mut active = ev.into_active_model();
            active.attempts = ActiveValue::set(new_attempts);
            active.status = ActiveValue::set("failed".to_string());
            active.last_error = ActiveValue::set(Some("retry limit exceeded".to_string()));
            active.update(&ctx.db).await?;
            continue;
        }

        let rule = HrRules::find_by_id(&ev.action_key).one(&ctx.db).await?;
        let is_enabled = rule.map(|r| r.enabled).unwrap_or(false);

        if !is_enabled {
            let mut active = ev.into_active_model();
            active.attempts = ActiveValue::set(new_attempts);
            active.status = ActiveValue::set("complete".to_string());
            active.completed_at = ActiveValue::set(Some(now));
            active.update(&ctx.db).await?;
            continue;
        }

        if let Some(roles) = recipient_roles(&ev.action_key) {
            for role in roles {
                let active_users = Users::find()
                    .filter(users::Column::Role.eq(*role))
                    .filter(users::Column::Status.eq("active"))
                    .all(&ctx.db)
                    .await?;

                for u in active_users {
                    let notif_exists = HrNotifications::find()
                        .filter(hr_notifications::Column::EventId.eq(ev.event_id))
                        .filter(hr_notifications::Column::UserId.eq(u.id))
                        .one(&ctx.db)
                        .await?
                        .is_some();

                    if !notif_exists {
                        let notif = hr_notifications::ActiveModel {
                            id: ActiveValue::set(Uuid::new_v4()),
                            user_id: ActiveValue::set(u.id),
                            event_id: ActiveValue::set(ev.event_id),
                            rule_key: ActiveValue::set(ev.action_key.clone()),
                            title: ActiveValue::set(notification_title(&ev.action_key).to_string()),
                            body: ActiveValue::set(notification_body(&ev.payload)),
                            created_at: ActiveValue::set(now),
                            read_at: ActiveValue::set(None),
                        };
                        notif.insert(&ctx.db).await?;
                    }
                }
            }
        }

        let mut active = ev.into_active_model();
        active.attempts = ActiveValue::set(new_attempts);
        active.status = ActiveValue::set("complete".to_string());
        active.completed_at = ActiveValue::set(Some(now));
        active.update(&ctx.db).await?;
    }

    Ok(())
}

pub async fn record_failure(ctx: &AppContext, event_id: Uuid) -> Result<()> {
    let events = HrRuleEvents::find()
        .filter(hr_rule_events::Column::EventId.eq(event_id))
        .filter(hr_rule_events::Column::Status.eq("pending"))
        .all(&ctx.db)
        .await?;

    for ev in events {
        let mut active = ev.into_active_model();
        active.last_error = ActiveValue::set(Some("Rule action failed; see application logs.".to_string()));
        if active.attempts.as_ref() >= &5 {
            active.status = ActiveValue::set("failed".to_string());
        }
        active.update(&ctx.db).await?;
    }
    Ok(())
}

fn recipient_roles(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "employee.created" | "employee.updated" | "employee.archived" => Some(&["hr_admin"]),
        "leave.submitted" | "leave.decided" => Some(&["hr_admin", "manager"]),
        "expense.submitted" | "expense.decided" => Some(&["hr_admin", "finance"]),
        "candidate.stage_changed" => Some(&["hr_admin", "recruiter"]),
        "payroll.completed" => Some(&["hr_admin", "payroll_admin"]),
        _ => None,
    }
}

fn notification_title(key: &str) -> &'static str {
    match key {
        "employee.created" => "Employee record created",
        "employee.updated" => "Employee record updated",
        "employee.archived" => "Employee record archived",
        "leave.submitted" => "Leave request submitted",
        "leave.decided" => "Leave request decided",
        "expense.submitted" => "Expense claim submitted",
        "expense.decided" => "Expense claim decided",
        "candidate.stage_changed" => "Candidate stage changed",
        "payroll.completed" => "Payroll run completed",
        _ => "System event",
    }
}

fn notification_body(payload: &serde_json::Value) -> String {
    let record_type = payload
        .get("record_type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("record");
    let record_id = payload
        .get("record_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    format!("{record_type} {record_id} was processed by rule engine.")
}