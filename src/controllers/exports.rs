use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    authz::AuthUser,
    models::_entities::{
        hr_record_history::{self, Entity as HrRecordHistory},
        hr_record_unique_claims,
        hr_records::{self, Entity as HrRecords},
    },
    records,
};

const EXPORT_LIMIT: u64 = 10_000;

#[derive(Debug, Serialize, ToSchema)]
pub struct BackupDocument {
    pub format_version: u32,
    pub records: Vec<BackupRecord>,
    pub history: Vec<BackupHistory>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BackupRecord {
    pub id: Uuid,
    pub record_type: String,
    pub source_id: String,
    pub data: serde_json::Value,
    pub archived_at: Option<String>,
    pub created_by: i64,
    pub updated_by: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BackupImport {
    pub format_version: u32,
    pub records: Vec<BackupRecord>,
    #[serde(default)]
    pub history: Vec<BackupHistory>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BackupHistory {
    pub record_id: Uuid,
    pub action: String,
    pub source_actor_user_id: i64,
    pub before_data: Option<serde_json::Value>,
    pub after_data: Option<serde_json::Value>,
    pub created_at: String,
}

fn require_export_access(auth: &AuthUser) -> Result<()> {
    auth.require("backup:manage").map(|_| ())
}

fn no_store(response: &mut Response) {
    response.headers_mut().insert(
        "cache-control",
        "no-store, private".parse().expect("static header"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
}

fn payload_too_large() -> Error {
    Error::CustomError(
        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
        loco_rs::controller::ErrorDetail::new(
            "export_too_large",
            "Export exceeds the 10 MB download limit.",
        ),
    )
}

fn json_download(value: &impl Serialize) -> Result<Response> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::InternalServerError)?;
    if bytes.len() > 10_000_000 {
        return Err(payload_too_large());
    }
    let mut response = Response::builder()
        .header("content-type", "application/json; charset=utf-8")
        .header(
            "content-disposition",
            "attachment; filename=brightcone-backup.json",
        )
        .body(axum::body::Body::from(bytes))
        .map_err(|_| Error::InternalServerError)?;
    no_store(&mut response);
    Ok(response)
}

fn csv_cell(val: &str) -> String {
    if val.contains(',') || val.contains('\"') || val.contains('\n') || val.contains('\r') {
        format!("\"{}\"", val.replace('\"', "\"\""))
    } else {
        val.to_string()
    }
}

#[utoipa::path(
    get,
    path = "/api/records/export.csv",
    security(("bearer" = [])),
    responses((status = 200, description = "CSV export"))
)]
#[debug_handler]
pub async fn csv(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Query(query): Query<super::records::ListQuery>,
) -> Result<Response> {
    require_export_access(&auth)?;
    let mut select = HrRecords::find().order_by_desc(hr_records::Column::CreatedAt);

    if !query.include_archived.unwrap_or(false) {
        select = select.filter(hr_records::Column::ArchivedAt.is_null());
    }

    let rows = select.limit(EXPORT_LIMIT).all(&ctx.db).await?;

    let mut body =
        String::from("id,record_type,source_id,data,archived_at,created_at,updated_at\r\n");

    for r in rows {
        let values = [
            r.id.to_string(),
            r.record_type,
            r.source_id,
            r.data.to_string(),
            r.archived_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            r.created_at.to_rfc3339(),
            r.updated_at.to_rfc3339(),
        ];
        let line = values
            .iter()
            .map(|v| csv_cell(v))
            .collect::<Vec<_>>()
            .join(",");
        body.push_str(&line);
        body.push_str("\r\n");

        if body.len() > 10_000_000 {
            return Err(payload_too_large());
        }
    }

    let mut response = Response::builder()
        .header("content-type", "text/csv; charset=utf-8")
        .header(
            "content-disposition",
            "attachment; filename=brightcone-records.csv",
        )
        .body(axum::body::Body::from(body))
        .map_err(|_| Error::string("Could not generate export."))?;
    no_store(&mut response);
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/backup",
    security(("bearer" = [])),
    responses((status = 200, body = BackupDocument))
)]
#[debug_handler]
pub async fn backup(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    require_export_access(&auth)?;

    let record_count = HrRecords::find().count(&ctx.db).await?;
    let history_count = HrRecordHistory::find().count(&ctx.db).await?;

    if record_count > EXPORT_LIMIT || history_count > 100_000 {
        return Err(payload_too_large());
    }

    let records = HrRecords::find()
        .order_by_asc(hr_records::Column::Id)
        .limit(EXPORT_LIMIT)
        .all(&ctx.db)
        .await?;

    let history = HrRecordHistory::find()
        .order_by_asc(hr_record_history::Column::Id)
        .limit(100_000)
        .all(&ctx.db)
        .await?;

    let backup_records: Vec<BackupRecord> = records
        .into_iter()
        .map(|r| BackupRecord {
            id: r.id,
            record_type: r.record_type,
            source_id: r.source_id,
            data: r.data,
            archived_at: r.archived_at.map(|t| t.to_rfc3339()),
            created_by: r.created_by.unwrap_or(0),
            updated_by: r.updated_by.unwrap_or(0),
        })
        .collect();

    let backup_history: Vec<BackupHistory> = history
        .into_iter()
        .map(|h| BackupHistory {
            record_id: h.record_id,
            action: h.action,
            source_actor_user_id: h.source_actor_user_id.unwrap_or(h.actor_user_id),
            before_data: h.before_data,
            after_data: h.after_data,
            created_at: h.created_at.to_rfc3339(),
        })
        .collect();

    json_download(&BackupDocument {
        format_version: 2,
        records: backup_records,
        history: backup_history,
    })
}

#[utoipa::path(
    post,
    path = "/api/backup/import",
    security(("bearer" = [])),
    request_body = BackupImport,
    responses((status = 200, description = "Import summary"))
)]
#[debug_handler]
pub async fn import(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Json(mut backup): Json<BackupImport>,
) -> Result<Response> {
    require_export_access(&auth)?;
    if !(1..=2).contains(&backup.format_version)
        || backup.records.len() > usize::try_from(EXPORT_LIMIT).unwrap_or(10_000)
        || backup.history.len() > 100_000
    {
        return Err(Error::BadRequest(
            "Unsupported backup version or record limit exceeded.".to_owned(),
        ));
    }

    for row in &mut backup.records {
        if row.source_id.trim().is_empty() || row.source_id.len() > 128 {
            return Err(Error::BadRequest(
                "Backup contains an invalid source_id.".to_owned(),
            ));
        }
        row.data = records::validate_record(&row.record_type, &row.data)
            .map_err(|_| Error::BadRequest("Backup contains an invalid record.".to_owned()))?;
    }

    let actor = auth.user.id;
    let now = chrono::Local::now().fixed_offset();
    let total = backup.records.len();

    let txn = ctx.db.begin().await?;
    let mut imported_count = 0;

    for row in backup.records {
        let exists = HrRecords::find_by_id(row.id).one(&txn).await?;
        if exists.is_none() {
            let archived_at = row.archived_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.into())
            });

            let new_rec = hr_records::ActiveModel {
                id: ActiveValue::set(row.id),
                record_type: ActiveValue::set(row.record_type.clone()),
                source_id: ActiveValue::set(row.source_id),
                data: ActiveValue::set(row.data.clone()),
                archived_at: ActiveValue::set(archived_at),
                created_by: ActiveValue::set(Some(actor)),
                updated_by: ActiveValue::set(Some(actor)),
                created_at: ActiveValue::set(now),
                updated_at: ActiveValue::set(now),
            };
            new_rec.insert(&txn).await?;

            for claim in records::unique_claims(&row.record_type, &row.data) {
                let unique_claim = hr_record_unique_claims::ActiveModel {
                    record_type: ActiveValue::set(row.record_type.clone()),
                    field_key: ActiveValue::set(claim.field_key),
                    normalized_value: ActiveValue::set(claim.normalized_value),
                    record_id: ActiveValue::set(row.id),
                };
                let _ = unique_claim.insert(&txn).await;
            }
            imported_count += 1;
        }
    }

    for h in backup.history {
        let created_at = chrono::DateTime::parse_from_rfc3339(&h.created_at)
            .ok()
            .unwrap_or(now);

        let new_hist = hr_record_history::ActiveModel {
            record_id: ActiveValue::set(h.record_id),
            action: ActiveValue::set(h.action),
            actor_user_id: ActiveValue::set(actor),
            before_data: ActiveValue::set(h.before_data),
            after_data: ActiveValue::set(h.after_data),
            created_at: ActiveValue::set(created_at),
            source_actor_user_id: ActiveValue::set(Some(h.source_actor_user_id)),
            ..Default::default()
        };
        let _ = new_hist.insert(&txn).await;
    }

    txn.commit().await?;

    format::json(serde_json::json!({
        "status": "ok",
        "total": total,
        "imported": imported_count,
    }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/records/export.csv", get(csv))
        .add("/backup", get(backup))
        .add("/backup/import", post(import))
}
