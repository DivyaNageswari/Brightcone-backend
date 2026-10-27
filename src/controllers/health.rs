use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is up and database is responding"),
    )
)]
#[debug_handler]
pub async fn health(State(ctx): State<AppContext>) -> Result<Response> {
    let db_status = match ctx.db.execute_unprepared("SELECT 1;").await {
        Ok(_) => "ok",
        Err(_) => "error",
    };
    format::json(serde_json::json!({
        "status": "ok",
        "database": db_status
    }))
}

pub fn routes() -> Routes {
    Routes::new().prefix("/api").add("/health", get(health))
}
