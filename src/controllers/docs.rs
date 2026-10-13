use loco_rs::prelude::*;
use utoipa::OpenApi;

use crate::openapi::ApiDoc;

/// The machine-readable list of endpoints. Only mounted outside production (see `app.rs`).
#[debug_handler]
async fn openapi_json() -> Result<Response> {
    format::json(ApiDoc::openapi())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/openapi.json")
        .add("/", get(openapi_json))
}
