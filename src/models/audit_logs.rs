use chrono::{DateTime, Utc};
use diesel::prelude::QueryableByName;
use diesel::sql_types::{BigInt, Nullable, Text, Timestamptz};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, QueryableByName)]
pub struct Model {
    #[diesel(sql_type = BigInt)]
    pub id: i64,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub actor_user_id: Option<i64>,
    #[diesel(sql_type = Nullable<Text>)]
    pub actor_email: Option<String>,
    #[diesel(sql_type = Text)]
    pub action: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub target_type: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub target_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub summary: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub ip: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub request_id: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
}
