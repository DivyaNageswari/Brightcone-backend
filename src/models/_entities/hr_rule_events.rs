use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_rule_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub action_key: String,
    pub status: String,
    pub attempts: i32,
    pub payload: Json,
    pub last_error: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub completed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}