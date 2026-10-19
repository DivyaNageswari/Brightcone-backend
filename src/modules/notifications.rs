use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub message: String,
    pub category: String, // "payroll", "expense", "helpdesk", "announcement", "system"
    pub read: bool,
    pub created_at: String,
    pub action_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateNotificationDto {
    pub user_id: Option<String>,
    pub title: String,
    pub message: String,
    pub category: String,
    pub action_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnreadCountResponse {
    pub unread_count: usize,
}

#[derive(Clone, Default)]
pub struct NotificationsStore {
    pub notifications: Arc<RwLock<Vec<Notification>>>,
}

impl NotificationsStore {
    pub fn new() -> Self {
        let default_items = vec![
            Notification {
                id: "n-1".to_string(),
                user_id: "current-user".to_string(),
                title: "Payroll Ready for Review".to_string(),
                message: "September 2026 payroll run is at Step 3: Review Variances.".to_string(),
                category: "payroll".to_string(),
                read: false,
                created_at: "10 mins ago".to_string(),
                action_url: Some("/payroll/admin".to_string()),
            },
            Notification {
                id: "n-2".to_string(),
                user_id: "current-user".to_string(),
                title: "Expense Claim Approved".to_string(),
                message: "Your claim of Rs 4,200 for Client Lunch was approved by Finance."
                    .to_string(),
                category: "expense".to_string(),
                read: false,
                created_at: "1 hour ago".to_string(),
                action_url: Some("/payroll/expenses".to_string()),
            },
            Notification {
                id: "n-3".to_string(),
                user_id: "current-user".to_string(),
                title: "Company All-Hands Meeting".to_string(),
                message: "Town hall scheduled for Friday at 4:00 PM. RSVP open.".to_string(),
                category: "announcement".to_string(),
                read: true,
                created_at: "Yesterday".to_string(),
                action_url: Some("/engagement/feed".to_string()),
            },
        ];

        Self {
            notifications: Arc::new(RwLock::new(default_items)),
        }
    }
}

pub async fn list_notifications(
    State(store): State<NotificationsStore>,
) -> Json<Vec<Notification>> {
    let items = store.notifications.read().await;
    Json(items.clone())
}

pub async fn get_unread_count(
    State(store): State<NotificationsStore>,
) -> Json<UnreadCountResponse> {
    let items = store.notifications.read().await;
    let count = items.iter().filter(|n| !n.read).count();
    Json(UnreadCountResponse {
        unread_count: count,
    })
}

pub async fn mark_read(
    State(store): State<NotificationsStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut items = store.notifications.write().await;
    if let Some(item) = items.iter_mut().find(|n| n.id == id) {
        item.read = true;
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

pub async fn mark_all_read(State(store): State<NotificationsStore>) -> impl IntoResponse {
    let mut items = store.notifications.write().await;
    for item in items.iter_mut() {
        item.read = true;
    }
    StatusCode::OK
}

pub async fn create_notification(
    State(store): State<NotificationsStore>,
    Json(dto): Json<CreateNotificationDto>,
) -> (StatusCode, Json<Notification>) {
    let mut items = store.notifications.write().await;
    let new_item = Notification {
        id: format!("n-{}", Uuid::new_v4()),
        user_id: dto.user_id.unwrap_or_else(|| "current-user".to_string()),
        title: dto.title,
        message: dto.message,
        category: dto.category,
        read: false,
        created_at: Utc::now().to_rfc3339(),
        action_url: dto.action_url,
    };
    items.insert(0, new_item.clone());
    (StatusCode::CREATED, Json(new_item))
}
