use crate::models::_entities::hr_records;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordInput {
    pub source_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordUpdateInput {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Record {
    pub id: Uuid,
    pub record_type: String,
    pub source_id: String,
    pub data: serde_json::Value,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<hr_records::Model> for Record {
    fn from(m: hr_records::Model) -> Self {
        Self {
            id: m.id,
            record_type: m.record_type,
            source_id: m.source_id,
            data: m.data,
            archived_at: m.archived_at.map(|t| t.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordPage {
    pub items: Vec<Record>,
}

pub type HistoryEntry = RecordHistory;

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordHistory {
    pub id: i64,
    pub record_id: Uuid,
    pub action: String,
    pub actor_user_id: i64,
    pub before_data: Option<serde_json::Value>,
    pub after_data: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<u64>,
    pub include_archived: Option<bool>,
}
