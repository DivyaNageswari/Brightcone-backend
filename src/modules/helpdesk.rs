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
pub struct TicketReply {
    pub id: String,
    pub author_name: String,
    pub author_role: String, // "Employee", "HR Partner"
    pub message: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub subject: String,
    pub category: String, // "Payroll & tax", "Leave & attendance", "Benefits & insurance", "IT & assets", "General HR"
    pub priority: String, // "Low", "Medium", "High", "Urgent"
    pub status: String,   // "Open", "In progress", "Resolved"
    pub assigned_to: Option<String>,
    pub created_at: String,
    pub replies: Vec<TicketReply>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateTicketDto {
    pub employee_id: Option<String>,
    pub employee_name: Option<String>,
    pub subject: String,
    pub category: String,
    pub priority: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AddReplyDto {
    pub author_name: String,
    pub author_role: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AssignTicketDto {
    pub assigned_to: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateStatusDto {
    pub status: String,
}

#[derive(Clone, Default)]
pub struct HelpdeskStore {
    pub tickets: Arc<RwLock<Vec<Ticket>>>,
}

impl HelpdeskStore {
    pub fn new() -> Self {
        let initial_tickets = vec![
            Ticket {
                id: "T-101".to_string(),
                employee_id: "EMP-001".to_string(),
                employee_name: "Ananya Rao".to_string(),
                subject: "Salary Account Bank Change".to_string(),
                category: "Payroll & tax".to_string(),
                priority: "High".to_string(),
                status: "In progress".to_string(),
                assigned_to: Some("Karthik Nair".to_string()),
                created_at: "2026-10-13T10:00:00Z".to_string(),
                replies: vec![
                    TicketReply {
                        id: "r-1".to_string(),
                        author_name: "Ananya Rao".to_string(),
                        author_role: "Employee".to_string(),
                        message: "I have updated my primary salary account to HDFC Bank. Attached the cancelled cheque for IFSC verification.".to_string(),
                        created_at: "2026-10-13T10:00:00Z".to_string(),
                    },
                    TicketReply {
                        id: "r-2".to_string(),
                        author_name: "Karthik Nair".to_string(),
                        author_role: "HR Partner".to_string(),
                        message: "Received your document Ananya. Forwarding to the finance payroll team for verification with bank portal.".to_string(),
                        created_at: "2026-10-13T14:30:00Z".to_string(),
                    },
                ],
            },
            Ticket {
                id: "T-102".to_string(),
                employee_id: "EMP-002".to_string(),
                employee_name: "Rohan Patel".to_string(),
                subject: "Parental Health Insurance Add-on query".to_string(),
                category: "Benefits & insurance".to_string(),
                priority: "Medium".to_string(),
                status: "Open".to_string(),
                assigned_to: None,
                created_at: "2026-10-14T11:20:00Z".to_string(),
                replies: vec![
                    TicketReply {
                        id: "r-3".to_string(),
                        author_name: "Rohan Patel".to_string(),
                        author_role: "Employee".to_string(),
                        message: "Is the deadline for senior citizen parents copay waiver extended till end of month?".to_string(),
                        created_at: "2026-10-14T11:20:00Z".to_string(),
                    },
                ],
            },
            Ticket {
                id: "T-103".to_string(),
                employee_id: "EMP-003".to_string(),
                employee_name: "Sneha Reddy".to_string(),
                subject: "Loss of Pay deduction discrepancy".to_string(),
                category: "Payroll & tax".to_string(),
                priority: "Urgent".to_string(),
                status: "Resolved".to_string(),
                assigned_to: Some("Ananya Rao".to_string()),
                created_at: "2026-10-09T08:45:00Z".to_string(),
                replies: vec![
                    TicketReply {
                        id: "r-4".to_string(),
                        author_name: "Sneha Reddy".to_string(),
                        author_role: "Employee".to_string(),
                        message: "My approved medical leave on Oct 2nd was deducted as LOP in preliminary register.".to_string(),
                        created_at: "2026-10-09T08:45:00Z".to_string(),
                    },
                    TicketReply {
                        id: "r-5".to_string(),
                        author_name: "Ananya Rao".to_string(),
                        author_role: "HR Partner".to_string(),
                        message: "Verified medical certificate and regularized in attendance ledger. LOP removed.".to_string(),
                        created_at: "2026-10-09T16:10:00Z".to_string(),
                    },
                ],
            },
        ];

        Self {
            tickets: Arc::new(RwLock::new(initial_tickets)),
        }
    }
}

pub async fn list_tickets(State(store): State<HelpdeskStore>) -> Json<Vec<Ticket>> {
    let tickets = store.tickets.read().await;
    Json(tickets.clone())
}

pub async fn get_ticket(
    State(store): State<HelpdeskStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let tickets = store.tickets.read().await;
    if let Some(t) = tickets.iter().find(|t| t.id == id) {
        (StatusCode::OK, Json(serde_json::json!(t)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Ticket not found." })),
        )
    }
}

pub async fn create_ticket(
    State(store): State<HelpdeskStore>,
    Json(dto): Json<CreateTicketDto>,
) -> (StatusCode, Json<Ticket>) {
    let mut tickets = store.tickets.write().await;
    let author_name = dto
        .employee_name
        .clone()
        .unwrap_or_else(|| "Ananya Rao".to_string());
    let initial_msg = dto.message.clone();

    let new_ticket = Ticket {
        id: format!("T-{}", Uuid::new_v4().to_string()[..5].to_uppercase()),
        employee_id: dto.employee_id.unwrap_or_else(|| "EMP-001".to_string()),
        employee_name: author_name.clone(),
        subject: dto.subject,
        category: dto.category,
        priority: dto.priority,
        status: "Open".to_string(),
        assigned_to: None,
        created_at: Utc::now().to_rfc3339(),
        replies: vec![TicketReply {
            id: format!("r-{}", &Uuid::new_v4().to_string()[..6]),
            author_name,
            author_role: "Employee".to_string(),
            message: initial_msg,
            created_at: Utc::now().to_rfc3339(),
        }],
    };

    tickets.insert(0, new_ticket.clone());
    (StatusCode::CREATED, Json(new_ticket))
}

pub async fn add_reply(
    State(store): State<HelpdeskStore>,
    Path(id): Path<String>,
    Json(dto): Json<AddReplyDto>,
) -> impl IntoResponse {
    let mut tickets = store.tickets.write().await;
    if let Some(ticket) = tickets.iter_mut().find(|t| t.id == id) {
        let reply = TicketReply {
            id: format!("r-{}", &Uuid::new_v4().to_string()[..6]),
            author_name: dto.author_name,
            author_role: dto.author_role,
            message: dto.message,
            created_at: Utc::now().to_rfc3339(),
        };
        ticket.replies.push(reply.clone());
        (StatusCode::CREATED, Json(serde_json::json!(ticket)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Ticket not found." })),
        )
    }
}

pub async fn assign_ticket(
    State(store): State<HelpdeskStore>,
    Path(id): Path<String>,
    Json(dto): Json<AssignTicketDto>,
) -> impl IntoResponse {
    let mut tickets = store.tickets.write().await;
    if let Some(ticket) = tickets.iter_mut().find(|t| t.id == id) {
        ticket.assigned_to = Some(dto.assigned_to);
        if ticket.status == "Open" {
            ticket.status = "In progress".to_string();
        }
        (StatusCode::OK, Json(serde_json::json!(ticket)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Ticket not found." })),
        )
    }
}

pub async fn update_status(
    State(store): State<HelpdeskStore>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateStatusDto>,
) -> impl IntoResponse {
    let mut tickets = store.tickets.write().await;
    if let Some(ticket) = tickets.iter_mut().find(|t| t.id == id) {
        ticket.status = dto.status;
        (StatusCode::OK, Json(serde_json::json!(ticket)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Ticket not found." })),
        )
    }
}
