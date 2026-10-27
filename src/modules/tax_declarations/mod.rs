pub mod handlers;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};
pub use handlers::*;
use std::sync::Arc;
use tokio::sync::RwLock;
pub use types::*;

#[derive(Clone)]
pub struct TaxDeclarationsStore {
    pub declarations: Arc<RwLock<Vec<TaxDeclaration>>>,
}

fn seed(
    id: &str,
    emp: &str,
    name: &str,
    dept: &str,
    regime: &str,
    status: &str,
    s80c: f64,
    s80d: f64,
    snps: f64,
    s24: f64,
    hra: f64,
    ver: f64,
) -> TaxDeclaration {
    let tot = s80c + s80d + snps + s24 + hra;
    let now = "2026-10-15T10:00:00Z".to_string();
    TaxDeclaration {
        id: id.into(),
        employee_id: emp.into(),
        employee_name: name.into(),
        department: dept.into(),
        financial_year: "FY 2026-27".into(),
        tax_regime: regime.into(),
        status: status.into(),
        section_80c: s80c,
        section_80d: s80d,
        section_80ccd_nps: snps,
        section_24_home_loan: s24,
        hra_annual_rent: hra,
        hra_landlord_pan: None,
        hra_landlord_name: None,
        hra_rental_address: None,
        section_80e_education_loan: 0.0,
        section_80g_donations: 0.0,
        other_exemptions: 0.0,
        total_declared: tot,
        total_verified: ver,
        proof_attachments_count: if tot > 0.0 { 2 } else { 0 },
        rejection_reason: None,
        verification_notes: None,
        submitted_at: if status != "Draft" {
            Some(now.clone())
        } else {
            None
        },
        verified_at: if status == "Verified" {
            Some(now.clone())
        } else {
            None
        },
        created_at: now.clone(),
        updated_at: now,
    }
}

impl TaxDeclarationsStore {
    pub fn new() -> Self {
        let initial = vec![
            seed(
                "TD-2026-001",
                "BC-1001",
                "Ananya Rao",
                "People Ops",
                "Old",
                "Submitted",
                150_000.0,
                25_000.0,
                50_000.0,
                0.0,
                180_000.0,
                0.0,
            ),
            seed(
                "TD-2026-002",
                "BC-1003",
                "Karthik Reddy",
                "Engineering",
                "Old",
                "Verified",
                150_000.0,
                35_000.0,
                50_000.0,
                200_000.0,
                0.0,
                435_000.0,
            ),
            seed(
                "TD-2026-003",
                "BC-1002",
                "Naresh Vemparala",
                "Operations",
                "New",
                "Verified",
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ),
            seed(
                "TD-2026-004",
                "BC-1004",
                "Priya Menon",
                "Product",
                "Old",
                "Draft",
                120_000.0,
                15_000.0,
                0.0,
                0.0,
                144_000.0,
                0.0,
            ),
        ];
        Self {
            declarations: Arc::new(RwLock::new(initial)),
        }
    }
}

impl Default for TaxDeclarationsStore {
    fn default() -> Self {
        Self::new()
    }
}

pub fn router(store: TaxDeclarationsStore) -> Router {
    Router::new()
        .route(
            "/api/tax-declarations",
            get(list_declarations).post(create_declaration),
        )
        .route("/api/tax-declarations/summary", get(get_summary))
        .route(
            "/api/tax-declarations/employee/{employee_id}",
            get(get_employee_declaration),
        )
        .route(
            "/api/tax-declarations/{id}",
            get(get_declaration)
                .put(update_declaration)
                .delete(delete_declaration),
        )
        .route(
            "/api/tax-declarations/{id}/verify",
            post(verify_declaration),
        )
        .route(
            "/api/tax-declarations/{id}/reject",
            post(reject_declaration),
        )
        .with_state(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tax_declarations_store() {
        let store = TaxDeclarationsStore::new();
        let list = store.declarations.read().await;
        assert_eq!(list.len(), 4);
        assert_eq!(list[0].employee_id, "BC-1001");
        assert_eq!(list[0].tax_regime, "Old");
        assert_eq!(list[0].total_declared, 410_000.0);
    }
}
