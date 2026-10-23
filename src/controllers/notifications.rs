use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    authz::AuthUser,
    models::_entities::hr_notifications::{self, Entity as HrNotifications},
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub rule_key: String,
    pub title: String,
    pub body: String,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationPage {
    pub items: Vec<Notification>,
    pub unread_count: i64,
}

#[utoipa::path(
    get,
    path = "/api/notifications",
    security(("bearer" = [])),
    responses((status = 200, body = NotificationPage))
)]
#[debug_handler]
pub async fn list(
    auth: AuthUser,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let unread_count = HrNotifications::find()
        .filter(hr_notifications::Column::UserId.eq(auth.user.id))
        .filter(hr_notifications::Column::ReadAt.is_null())
        .count(&ctx.db)
        .await? as i64;

    let rows = HrNotifications::find()
        .filter(hr_notifications::Column::UserId.eq(auth.user.id))
        .order_by_desc(hr_notifications::Column::CreatedAt)
        .limit(50)
        .all(&ctx.db)
        .await?;

    let items = rows
        .into_iter()
        .map(|r| Notification {
            id: r.id,
            rule_key: r.rule_key,
            title: r.title,
            body: r.body,
            created_at: r.created_at.to_rfc3339(),
            read_at: r.read_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    format::json(NotificationPage {
        items,
        unread_count,
    })
}

#[utoipa::path(
    post,
    path = "/api/notifications/{id}/read",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Notification ID")),
    responses((status = 200, description = "Marked as read"))
)]
#[debug_handler]
pub async fn mark_read(
    auth: AuthUser,
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
) -> Result<Response> {
    let found = HrNotifications::find_by_id(id)
        .filter(hr_notifications::Column::UserId.eq(auth.user.id))
        .one(&ctx.db)
        .await?;

    if let Some(row) = found {
        let mut active = row.into_active_model();
        active.read_at = ActiveValue::set(Some(chrono::Local::now().fixed_offset()));
        active.update(&ctx.db).await?;
    }

    format::json(serde_json::json!({ "status": "ok" }))
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/notifications")
        .add("/", get(list))
        .add("/{id}/read", post(mark_read))
}