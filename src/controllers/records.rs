use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use uuid::Uuid;

use crate::{
    authz::AuthUser,
    models::_entities::{
        hr_record_history::{self, Entity as HrRecordHistory},
        hr_record_unique_claims::{self, Entity as HrRecordUniqueClaims},
        hr_records::{self, Entity as HrRecords},
    },
    records,
    rule_engine::{self, RuleEventPayload},
};

#[path = "records/types.rs"]
mod types;
pub use types::*;

fn require_hr(auth: &AuthUser) -> Result<()> {
    auth.require("records:manage").map(|_| ())
}

fn validate_type(record_type: &str) -> Result<()> {
    records::is_registered(record_type)
        .then_some(())
        .ok_or_else(|| Error::BadRequest("Unknown record type.".to_owned()))
}

fn validate_input(record_type: &str, input: &RecordInput) -> Result<serde_json::Value> {
    if input.source_id.trim().is_empty() || input.source_id.len() > 128 {
        return Err(Error::BadRequest(
            "source_id must be 1 to 128 characters.".to_owned(),
        ));
    }
    records::validate_record(record_type, &input.data).map_err(Error::BadRequest)
}

#[utoipa::path(
    get,
    path = "/api/records/{record_type}",
    security(("bearer" = [])),
    params(
        ("record_type" = String, Path),
        ("limit" = Option<u64>, Query),
        ("include_archived" = Option<bool>, Query)
    ),
    responses((status = 200, body = RecordPage))
)]
#[debug_handler]
pub async fn list(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(kind): Path<String>,
    Query(query): Query<ListQuery>,
) -> Result<Response> {
    require_hr(&auth)?;
    validate_type(&kind)?;
    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let mut select = HrRecords::find()
        .filter(hr_records::Column::RecordType.eq(&kind))
        .order_by_desc(hr_records::Column::CreatedAt)
        .limit(limit);

    if !query.include_archived.unwrap_or(false) {
        select = select.filter(hr_records::Column::ArchivedAt.is_null());
    }

    let rows = select.all(&ctx.db).await?;
    let items = rows.into_iter().map(Record::from).collect();
    format::json(RecordPage { items })
}

#[utoipa::path(
    post,
    path = "/api/records/{record_type}",
    security(("bearer" = [])),
    request_body = RecordInput,
    responses((status = 201, body = Record))
)]
#[debug_handler]
pub async fn create(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(kind): Path<String>,
    Json(input): Json<RecordInput>,
) -> Result<Response> {
    require_hr(&auth)?;
    validate_type(&kind)?;
    let data = validate_input(&kind, &input)?;
    let source_id = input.source_id;
    let actor = auth.user.id;
    let id = Uuid::new_v4();
    let now = chrono::Local::now().fixed_offset();

    let txn = ctx.db.begin().await?;

    let exists = HrRecords::find()
        .filter(hr_records::Column::RecordType.eq(&kind))
        .filter(hr_records::Column::SourceId.eq(&source_id))
        .one(&txn)
        .await?;

    if exists.is_some() {
        return Err(Error::CustomError(
            axum::http::StatusCode::CONFLICT,
            loco_rs::controller::ErrorDetail::new("conflict", "Record source_id already exists."),
        ));
    }

    let record = hr_records::ActiveModel {
        id: ActiveValue::set(id),
        record_type: ActiveValue::set(kind.clone()),
        source_id: ActiveValue::set(source_id),
        data: ActiveValue::set(data.clone()),
        archived_at: ActiveValue::set(None),
        created_by: ActiveValue::set(Some(actor)),
        updated_by: ActiveValue::set(Some(actor)),
        created_at: ActiveValue::set(now),
        updated_at: ActiveValue::set(now),
    };
    let saved_record = record.insert(&txn).await?;

    let hist = hr_record_history::ActiveModel {
        record_id: ActiveValue::set(id),
        action: ActiveValue::set("created".to_string()),
        actor_user_id: ActiveValue::set(actor),
        before_data: ActiveValue::set(None),
        after_data: ActiveValue::set(Some(data.clone())),
        created_at: ActiveValue::set(now),
        source_actor_user_id: ActiveValue::set(Some(actor)),
        ..Default::default()
    };
    hist.insert(&txn).await?;

    for claim in records::unique_claims(&kind, &data) {
        let unique_claim = hr_record_unique_claims::ActiveModel {
            record_type: ActiveValue::set(kind.clone()),
            field_key: ActiveValue::set(claim.field_key),
            normalized_value: ActiveValue::set(claim.normalized_value),
            record_id: ActiveValue::set(id),
        };
        unique_claim.insert(&txn).await?;
    }

    let event_id = if let Some(key) = rule_engine::event_key(&kind, "created", None, &data) {
        rule_engine::record_event(
            &txn,
            key,
            &RuleEventPayload {
                record_id: id,
                record_type: kind.clone(),
                actor_user_id: actor,
            },
        )
        .await?
    } else {
        None
    };

    txn.commit().await?;
    rule_engine::dispatch(&ctx, event_id).await;
    format::render()
        .status(201)
        .json(Record::from(saved_record))
}

async fn change(
    auth: AuthUser,
    ctx: AppContext,
    id: Uuid,
    input: Option<serde_json::Value>,
    action: &'static str,
) -> Result<Response> {
    require_hr(&auth)?;
    let actor = auth.user.id;
    let now = chrono::Local::now().fixed_offset();

    let txn = ctx.db.begin().await?;

    let found = HrRecords::find_by_id(id).one(&txn).await?;
    let Some(old) = found else {
        return Err(Error::NotFound);
    };

    let new_data = match input {
        Some(value) => {
            records::validate_record(&old.record_type, &value).map_err(Error::BadRequest)?
        }
        None => old.data.clone(),
    };

    if action == "updated" {
        HrRecordUniqueClaims::delete_many()
            .filter(hr_record_unique_claims::Column::RecordId.eq(id))
            .exec(&txn)
            .await?;

        for claim in records::unique_claims(&old.record_type, &new_data) {
            let unique_claim = hr_record_unique_claims::ActiveModel {
                record_type: ActiveValue::set(old.record_type.clone()),
                field_key: ActiveValue::set(claim.field_key),
                normalized_value: ActiveValue::set(claim.normalized_value),
                record_id: ActiveValue::set(id),
            };
            unique_claim.insert(&txn).await?;
        }
    }

    let mut active = old.clone().into_active_model();
    active.updated_by = ActiveValue::set(Some(actor));
    active.updated_at = ActiveValue::set(now);

    match action {
        "updated" => {
            active.data = ActiveValue::set(new_data.clone());
        }
        "archived" => {
            active.archived_at = ActiveValue::set(Some(now));
        }
        "restored" => {
            active.archived_at = ActiveValue::set(None);
        }
        _ => {}
    }

    let updated = active.update(&txn).await?;

    let hist = hr_record_history::ActiveModel {
        record_id: ActiveValue::set(id),
        action: ActiveValue::set(action.to_string()),
        actor_user_id: ActiveValue::set(actor),
        before_data: ActiveValue::set(Some(old.data.clone())),
        after_data: ActiveValue::set(Some(new_data.clone())),
        created_at: ActiveValue::set(now),
        source_actor_user_id: ActiveValue::set(Some(actor)),
        ..Default::default()
    };
    hist.insert(&txn).await?;

    let event_id = if let Some(key) =
        rule_engine::event_key(&old.record_type, action, Some(&old.data), &new_data)
    {
        rule_engine::record_event(
            &txn,
            key,
            &RuleEventPayload {
                record_id: updated.id,
                record_type: old.record_type.clone(),
                actor_user_id: actor,
            },
        )
        .await?
    } else {
        None
    };

    txn.commit().await?;
    rule_engine::dispatch(&ctx, event_id).await;
    format::json(Record::from(updated))
}

#[utoipa::path(
    put,
    path = "/api/records/{id}",
    security(("bearer" = [])),
    request_body = RecordUpdateInput,
    responses((status = 200, body = Record))
)]
#[debug_handler]
pub async fn update(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
    Json(input): Json<RecordUpdateInput>,
) -> Result<Response> {
    change(auth, ctx, id, Some(input.data), "updated").await
}

#[utoipa::path(
    post,
    path = "/api/records/{id}/archive",
    security(("bearer" = [])),
    responses((status = 200, body = Record))
)]
#[debug_handler]
pub async fn archive(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    change(auth, ctx, id, None, "archived").await
}

#[utoipa::path(
    post,
    path = "/api/records/{id}/restore",
    security(("bearer" = [])),
    responses((status = 200, body = Record))
)]
#[debug_handler]
pub async fn restore(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    change(auth, ctx, id, None, "restored").await
}

#[utoipa::path(
    get,
    path = "/api/records/{id}/history",
    security(("bearer" = [])),
    responses((status = 200, body = Vec<RecordHistory>))
)]
#[debug_handler]
pub async fn history(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    require_hr(&auth)?;
    let rows = HrRecordHistory::find()
        .filter(hr_record_history::Column::RecordId.eq(id))
        .order_by_desc(hr_record_history::Column::Id)
        .all(&ctx.db)
        .await?;

    let items: Vec<RecordHistory> = rows
        .into_iter()
        .map(|r| RecordHistory {
            id: r.id,
            record_id: r.record_id,
            action: r.action,
            actor_user_id: r.actor_user_id,
            before_data: r.before_data,
            after_data: r.after_data,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

    format::json(items)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/records")
        .add("/{id}", get(list).post(create).put(update))
        .add("/{id}/archive", post(archive))
        .add("/{id}/restore", post(restore))
        .add("/{id}/history", get(history))
}
