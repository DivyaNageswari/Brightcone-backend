use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_record_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub record_id: Uuid,
    pub action: String,
    pub actor_user_id: i64,
    pub before_data: Option<Json>,
    pub after_data: Option<Json>,
    pub created_at: DateTimeWithTimeZone,
    pub source_actor_user_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}