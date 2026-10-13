use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Result of the health check. `database` is `"ok"` or `"down"`.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "ok")]
    pub status: String,
    #[schema(example = "ok")]
    pub database: String,
}

impl HealthResponse {
    #[must_use]
    pub fn new(database_up: bool) -> Self {
        Self {
            status: if database_up { "ok" } else { "degraded" }.to_owned(),
            database: if database_up { "ok" } else { "down" }.to_owned(),
        }
    }
}
