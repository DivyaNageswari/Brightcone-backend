use diesel::{
    sql_query,
    sql_types::{Bool, Jsonb, Text, Uuid as SqlUuid},
};
use diesel_async::{AsyncConnection, RunQueryDsl};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::Database,
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

pub async fn record_event<C>(
    conn: &mut C,
    key: &str,
    payload: &RuleEventPayload,
) -> diesel::QueryResult<Option<Uuid>>
where
    C: diesel_async::AsyncConnection<Backend = diesel::pg::Pg> + Send,
{
    if !RULES.contains(&key) {
        return Ok(None);
    }
    let event_id = Uuid::new_v4();
    let json = serde_json::to_value(payload).expect("rule event payload is serializable");
    let inserted = sql_query("INSERT INTO hr_rule_events(event_id, action_key, payload) SELECT $1,$2,$3 WHERE EXISTS (SELECT 1 FROM hr_rules WHERE rule_key=$2 AND enabled=true) RETURNING event_id")
        .bind::<SqlUuid,_>(event_id)
        .bind::<Text,_>(key)
        .bind::<Jsonb,_>(json)
        .get_result::<EventIdRow>(conn)
        .await;
    match inserted {
        Ok(row) => Ok(Some(row.event_id)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}

#[derive(diesel::QueryableByName)]
struct EventIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    event_id: Uuid,
}

#[derive(diesel::QueryableByName)]
struct PendingEvent {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    event_id: Uuid,
}

pub async fn dispatch(ctx: &AppContext, event_id: Option<Uuid>) {
    let Some(event_id) = event_id else { return };
    if let Err(error) = RuleDispatchWorker::perform_later(ctx, RuleDispatchArgs { event_id }).await
    {
        tracing::error!(%event_id, error = %error, "rule event remains in the durable outbox");
    }
}

pub async fn enqueue_pending(ctx: &AppContext) -> Result<()> {
    let db = Database::from_context(ctx)?;
    let mut conn = db.get().await?;
    let pending = sql_query(
        "SELECT event_id FROM hr_rule_events WHERE status='pending' ORDER BY created_at LIMIT 500",
    )
    .load::<PendingEvent>(&mut *conn)
    .await
    .map_err(Database::map_error)?;
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
    let db = Database::from_context(ctx)?;
    let mut conn = db.get().await?;
    let attempt = sql_query("UPDATE hr_rule_events SET attempts=attempts+1 WHERE event_id=$1 AND status='pending' RETURNING attempts")
        .bind::<SqlUuid,_>(event_id)
        .get_result::<AttemptRow>(&mut *conn).await;
    let attempt = match attempt {
        Ok(row) => row,
        Err(diesel::result::Error::NotFound) => return Ok(()),
        Err(error) => return Err(Database::map_error(error)),
    };
    if attempt.attempts > 5 {
        sql_query("UPDATE hr_rule_events SET status='failed', last_error='retry limit exceeded' WHERE event_id=$1")
            .bind::<SqlUuid,_>(event_id).execute(&mut *conn).await.map_err(Database::map_error)?;
        return Ok(());
    }
    conn.transaction::<(), diesel::result::Error, _>(async move |conn| {
        let event = sql_query("SELECT e.event_id, e.action_key, e.payload, r.enabled FROM hr_rule_events e JOIN hr_rules r ON r.rule_key=e.action_key WHERE e.event_id=$1 AND e.status='pending' FOR UPDATE OF e")
            .bind::<SqlUuid,_>(event_id).get_result::<EventRow>(&mut *conn).await?;
        if !event.enabled {
            sql_query("UPDATE hr_rule_events SET status='complete', completed_at=now() WHERE event_id=$1")
                .bind::<SqlUuid,_>(event_id).execute(&mut *conn).await?;
            return Ok(());
        }
        let recipients = recipient_roles(&event.action_key);
        if let Some(roles) = recipients {
            for role in roles {
                sql_query("INSERT INTO hr_notifications(user_id,event_id,rule_key,title,body) SELECT id,$1,$2,$3,$4 FROM users WHERE role=$5 AND status='active' ON CONFLICT(event_id,user_id) DO NOTHING")
                    .bind::<SqlUuid,_>(event.event_id)
                    .bind::<Text,_>(&event.action_key)
                    .bind::<Text,_>(notification_title(&event.action_key))
                    .bind::<Text,_>(notification_body(&event.payload))
                    .bind::<Text,_>(*role)
                    .execute(&mut *conn).await?;
            }
        }
        sql_query("UPDATE hr_rule_events SET status='complete', completed_at=now() WHERE event_id=$1")
            .bind::<SqlUuid,_>(event_id).execute(&mut *conn).await?;
        Ok(())
    }).await.map_err(Database::map_error)
}

pub async fn record_failure(ctx: &AppContext, event_id: Uuid) -> Result<()> {
    let db = Database::from_context(ctx)?;
    let mut conn = db.get().await?;
    sql_query("UPDATE hr_rule_events SET last_error='Rule action failed; see application logs.', status=CASE WHEN attempts >= 5 THEN 'failed' ELSE status END WHERE event_id=$1 AND status='pending'")
        .bind::<SqlUuid,_>(event_id)
        .execute(&mut *conn).await.map_err(Database::map_error)?;
    Ok(())
}

#[derive(diesel::QueryableByName)]
struct EventRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    event_id: Uuid,
    #[diesel(sql_type = Text)]
    action_key: String,
    #[diesel(sql_type = Jsonb)]
    payload: serde_json::Value,
    #[diesel(sql_type = Bool)]
    enabled: bool,
}

#[derive(diesel::QueryableByName)]
struct AttemptRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    attempts: i32,
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
        _ => "HR activity",
    }
}

fn notification_body(payload: &serde_json::Value) -> String {
    format!(
        "A {} record was updated. Reference: {}.",
        payload
            .get("record_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("HR"),
        payload
            .get("record_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unavailable")
    )
}

#[cfg(test)]
mod tests {
    use super::{event_key, recipient_roles};
    use serde_json::json;

    #[test]
    fn dispatches_only_defined_domain_transitions() {
        assert_eq!(
            event_key("employee", "created", None, &json!({})),
            Some("employee.created")
        );
        assert_eq!(
            event_key(
                "leave_request",
                "created",
                None,
                &json!({"status":"pending"})
            ),
            Some("leave.submitted")
        );
        assert_eq!(
            event_key(
                "leave_request",
                "updated",
                Some(&json!({"status":"pending"})),
                &json!({"status":"approved"})
            ),
            Some("leave.decided")
        );
        assert_eq!(
            event_key(
                "candidate",
                "updated",
                Some(&json!({"stage":"applied"})),
                &json!({"stage":"interview"})
            ),
            Some("candidate.stage_changed")
        );
        assert_eq!(
            event_key(
                "payroll_run",
                "updated",
                Some(&json!({"status":"draft"})),
                &json!({"status":"completed"})
            ),
            Some("payroll.completed")
        );
    }

    #[test]
    fn ignores_no_op_and_unsupported_transitions() {
        assert_eq!(
            event_key(
                "leave_request",
                "updated",
                Some(&json!({"status":"pending"})),
                &json!({"status":"pending"})
            ),
            None
        );
        assert_eq!(
            event_key("custom_record", "updated", None, &json!({})),
            None
        );
    }

    #[test]
    fn recipient_policy_keeps_payroll_and_recruiting_scopes_separate() {
        assert_eq!(
            recipient_roles("payroll.completed"),
            Some(&["hr_admin", "payroll_admin"][..])
        );
        assert_eq!(
            recipient_roles("candidate.stage_changed"),
            Some(&["hr_admin", "recruiter"][..])
        );
        assert_eq!(
            recipient_roles("expense.decided"),
            Some(&["hr_admin", "finance"][..])
        );
        assert_eq!(recipient_roles("unregistered.rule"), None);
    }
}
