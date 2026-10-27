pub mod expenses;
pub mod office_expenses;
pub mod payroll;
pub mod salary;
pub mod tax_declarations;

use axum::{
    routing::{get, post},
    Router,
};
use expenses::{approve_claim, create_claim, list_claims, pay_claim, reject_claim, ExpensesStore};
use office_expenses::{
    create_office_expense, get_analytics, list_office_expenses, OfficeExpensesStore,
};
use salary::{
    approve_revision, calculate_salary, create_structure, list_increment_history, list_revisions,
    list_structures, propose_revision, SalaryStore,
};
use tax_declarations::TaxDeclarationsStore;

pub fn domain_router() -> Router {
    let expenses_store = ExpensesStore::new();
    let office_expenses_store = OfficeExpensesStore::new();
    let salary_store = SalaryStore::new();
    let tax_declarations_store = TaxDeclarationsStore::new();

    let expenses_router = Router::new()
        .route("/api/expenses", get(list_claims).post(create_claim))
        .route("/api/expenses/{id}/approve", post(approve_claim))
        .route("/api/expenses/{id}/reject", post(reject_claim))
        .route("/api/expenses/{id}/pay", post(pay_claim))
        .with_state(expenses_store);

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

    let tax_declarations_router = tax_declarations::router(tax_declarations_store);

    Router::new()
        .merge(expenses_router)
        .merge(office_expenses_router)
        .merge(salary_router)
        .merge(payroll::router())
        .merge(tax_declarations_router)
}
