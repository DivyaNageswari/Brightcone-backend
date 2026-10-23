use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_record_unique_claims")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub record_type: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub field_key: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub normalized_value: String,
    pub record_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}