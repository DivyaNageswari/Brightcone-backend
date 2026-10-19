pub mod announcements;
pub mod expenses;
pub mod helpdesk;
pub mod notifications;
pub mod office_expenses;
pub mod salary;

use announcements::{
    create_announcement, delete_announcement, list_announcements, toggle_pin, update_announcement,
    AnnouncementsStore,
};
use axum::{
    routing::{get, post, put},
    Router,
};
use expenses::{approve_claim, create_claim, list_claims, pay_claim, reject_claim, ExpensesStore};
use helpdesk::{
    add_reply, assign_ticket, create_ticket, get_ticket, list_tickets, update_status, HelpdeskStore,
};
use notifications::{
    create_notification, get_unread_count, list_notifications, mark_all_read, mark_read,
    NotificationsStore,
};
use office_expenses::{
    create_office_expense, get_analytics, list_office_expenses, OfficeExpensesStore,
};
use salary::{
    approve_revision, calculate_salary, create_structure, list_increment_history, list_revisions,
    list_structures, propose_revision, SalaryStore,
};

pub fn domain_router() -> Router {
    let notifications_store = NotificationsStore::new();
    let expenses_store = ExpensesStore::new();
    let helpdesk_store = HelpdeskStore::new();
    let announcements_store = AnnouncementsStore::new();
    let office_expenses_store = OfficeExpensesStore::new();
    let salary_store = SalaryStore::new();

    let notifications_router = Router::new()
        .route(
            "/api/notifications",
            get(list_notifications).post(create_notification),
        )
        .route("/api/notifications/unread-count", get(get_unread_count))
        .route("/api/notifications/{id}/read", post(mark_read))
        .route("/api/notifications/mark-all-read", post(mark_all_read))
        .with_state(notifications_store);

    let expenses_router = Router::new()
        .route("/api/expenses", get(list_claims).post(create_claim))
        .route("/api/expenses/{id}/approve", post(approve_claim))
        .route("/api/expenses/{id}/reject", post(reject_claim))
        .route("/api/expenses/{id}/pay", post(pay_claim))
        .with_state(expenses_store);

    let helpdesk_router = Router::new()
        .route(
            "/api/helpdesk/tickets",
            get(list_tickets).post(create_ticket),
        )
        .route("/api/helpdesk/tickets/{id}", get(get_ticket))
        .route("/api/helpdesk/tickets/{id}/reply", post(add_reply))
        .route("/api/helpdesk/tickets/{id}/assign", post(assign_ticket))
        .route("/api/helpdesk/tickets/{id}/status", post(update_status))
        .with_state(helpdesk_store);

    let announcements_router = Router::new()
        .route(
            "/api/announcements",
            get(list_announcements).post(create_announcement),
        )
        .route(
            "/api/announcements/{id}",
            put(update_announcement).delete(delete_announcement),
        )
        .route("/api/announcements/{id}/toggle-pin", post(toggle_pin))
        .with_state(announcements_store);

    let office_expenses_router = Router::new()
        .route(
            "/api/office-expenses",
            get(list_office_expenses).post(create_office_expense),
        )
        .route("/api/office-expenses/analytics", get(get_analytics))
        .with_state(office_expenses_store);

    let salary_router = Router::new()
        .route("/api/salary/calculate", post(calculate_salary))
        .route(
            "/api/salary/structures",
            get(list_structures).post(create_structure),
        )
        .route(
            "/api/salary/revisions",
            get(list_revisions).post(propose_revision),
        )
        .route("/api/salary/revisions/{id}/approve", post(approve_revision))
        .route("/api/salary/increments", get(list_increment_history))
        .with_state(salary_store);

    Router::new()
        .merge(notifications_router)
        .merge(expenses_router)
        .merge(helpdesk_router)
        .merge(announcements_router)
        .merge(office_expenses_router)
        .merge(salary_router)
}
