use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use super::types::*;
use super::TaxDeclarationsStore;

pub async fn list_declarations(
    State(store): State<TaxDeclarationsStore>,
    Query(q): Query<DeclarationFilterQuery>,
) -> Json<Vec<TaxDeclaration>> {
    let list = store.declarations.read().await;
    let filtered = list
        .iter()
        .filter(|d| {
            q.employee_id
                .as_ref()
                .map_or(true, |e| d.employee_id.eq_ignore_ascii_case(e))
                && q.status
                    .as_ref()
                    .map_or(true, |s| d.status.eq_ignore_ascii_case(s))
                && q.regime
                    .as_ref()
                    .map_or(true, |r| d.tax_regime.eq_ignore_ascii_case(r))
        })
        .cloned()
        .collect();
    Json(filtered)
}

pub async fn get_declaration(
    State(store): State<TaxDeclarationsStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let list = store.declarations.read().await;
    match list.iter().find(|d| d.id == id) {
        Some(d) => (StatusCode::OK, Json(serde_json::json!(d))),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Tax declaration not found." })),
        ),
    }
}

pub async fn get_employee_declaration(
    State(store): State<TaxDeclarationsStore>,
    Path(emp): Path<String>,
) -> impl IntoResponse {
    let list = store.declarations.read().await;
    match list
        .iter()
        .find(|d| d.employee_id.eq_ignore_ascii_case(&emp))
    {
        Some(d) => (StatusCode::OK, Json(serde_json::json!(d))),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No declaration found." })),
        ),
    }
}

pub async fn get_summary(State(store): State<TaxDeclarationsStore>) -> Json<TaxDeclarationSummary> {
    let list = store.declarations.read().await;
    let (mut pending, mut verified, mut rejected, mut draft) = (0, 0, 0, 0);
    let (mut tot_decl, mut tot_ver, mut old_r, mut new_r) = (0.0, 0.0, 0, 0);

    for d in list.iter() {
        match d.status.as_str() {
            "Submitted" | "Pending" => pending += 1,
            "Verified" | "Approved" => verified += 1,
            "Rejected" => rejected += 1,
            _ => draft += 1,
        }
        if d.tax_regime.eq_ignore_ascii_case("old") {
            old_r += 1;
        } else {
            new_r += 1;
        }
        tot_decl += d.total_declared;
        tot_ver += d.total_verified;
    }
    Json(TaxDeclarationSummary {
        total_declarations: list.len(),
        pending_verification: pending,
        verified_count: verified,
        rejected_count: rejected,
        draft_count: draft,
        total_declared_amount: (tot_decl * 100.0).round() / 100.0,
        total_verified_amount: (tot_ver * 100.0).round() / 100.0,
        old_regime_count: old_r,
        new_regime_count: new_r,
    })
}

pub async fn create_declaration(
    State(store): State<TaxDeclarationsStore>,
    Json(dto): Json<CreateTaxDeclarationDto>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut list = store.declarations.write().await;
    let s80c = dto.section_80c.unwrap_or(0.0).min(150_000.0);
    let s80d = dto.section_80d.unwrap_or(0.0).min(100_000.0);
    let snps = dto.section_80ccd_nps.unwrap_or(0.0).min(50_000.0);
    let s24 = dto.section_24_home_loan.unwrap_or(0.0).min(200_000.0);
    let hra = dto.hra_annual_rent.unwrap_or(0.0);
    let s80e = dto.section_80e_education_loan.unwrap_or(0.0);
    let s80g = dto.section_80g_donations.unwrap_or(0.0);
    let oth = dto.other_exemptions.unwrap_or(0.0);
    let total = s80c + s80d + snps + s24 + hra + s80e + s80g + oth;
    let now = Utc::now().to_rfc3339();
    let status = dto.status.unwrap_or_else(|| "Submitted".into());
    let sub_at = if status == "Submitted" {
        Some(now.clone())
    } else {
        None
    };

    let declaration = TaxDeclaration {
        id: format!(
            "TD-2026-{}",
            &Uuid::new_v4().to_string()[..4].to_uppercase()
        ),
        employee_id: dto.employee_id.unwrap_or_else(|| "BC-1001".into()),
        employee_name: dto.employee_name.unwrap_or_else(|| "Ananya Rao".into()),
        department: dto.department.unwrap_or_else(|| "People Ops".into()),
        financial_year: dto.financial_year.unwrap_or_else(|| "FY 2026-27".into()),
        tax_regime: dto.tax_regime.unwrap_or_else(|| "Old".into()),
        status,
        section_80c: s80c,
        section_80d: s80d,
        section_80ccd_nps: snps,
        section_24_home_loan: s24,
        hra_annual_rent: hra,
        hra_landlord_pan: dto.hra_landlord_pan,
        hra_landlord_name: dto.hra_landlord_name,
        hra_rental_address: dto.hra_rental_address,
        section_80e_education_loan: s80e,
        section_80g_donations: s80g,
        other_exemptions: oth,
        total_declared: (total * 100.0).round() / 100.0,
        total_verified: 0.0,
        proof_attachments_count: dto.proof_attachments_count.unwrap_or(0),
        rejection_reason: None,
        verification_notes: None,
        submitted_at: sub_at,
        verified_at: None,
        created_at: now.clone(),
        updated_at: now,
    };
    list.insert(0, declaration.clone());
    (StatusCode::CREATED, Json(serde_json::json!(declaration)))
}

pub async fn update_declaration(
    State(store): State<TaxDeclarationsStore>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateTaxDeclarationDto>,
) -> impl IntoResponse {
    let mut list = store.declarations.write().await;
    if let Some(d) = list.iter_mut().find(|d| d.id == id) {
        if d.status == "Verified" {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Verified declarations cannot be edited." })),
            );
        }
        if let Some(r) = dto.tax_regime {
            d.tax_regime = r;
        }
        if let Some(v) = dto.section_80c {
            d.section_80c = v.min(150_000.0);
        }
        if let Some(v) = dto.section_80d {
            d.section_80d = v.min(100_000.0);
        }
        if let Some(v) = dto.section_80ccd_nps {
            d.section_80ccd_nps = v.min(50_000.0);
        }
        if let Some(v) = dto.section_24_home_loan {
            d.section_24_home_loan = v.min(200_000.0);
        }
        if let Some(v) = dto.hra_annual_rent {
            d.hra_annual_rent = v;
        }
        if let Some(v) = dto.hra_landlord_pan {
            d.hra_landlord_pan = Some(v);
        }
        if let Some(v) = dto.hra_landlord_name {
            d.hra_landlord_name = Some(v);
        }
        if let Some(v) = dto.hra_rental_address {
            d.hra_rental_address = Some(v);
        }
        if let Some(v) = dto.section_80e_education_loan {
            d.section_80e_education_loan = v;
        }
        if let Some(v) = dto.section_80g_donations {
            d.section_80g_donations = v;
        }
        if let Some(v) = dto.other_exemptions {
            d.other_exemptions = v;
        }
        if let Some(v) = dto.proof_attachments_count {
            d.proof_attachments_count = v;
        }
        if let Some(st) = dto.status {
            d.status = st;
            if d.status == "Submitted" && d.submitted_at.is_none() {
                d.submitted_at = Some(Utc::now().to_rfc3339());
            }
        }
        let total = d.section_80c
            + d.section_80d
            + d.section_80ccd_nps
            + d.section_24_home_loan
            + d.hra_annual_rent
            + d.section_80e_education_loan
            + d.section_80g_donations
            + d.other_exemptions;
        d.total_declared = (total * 100.0).round() / 100.0;
        d.updated_at = Utc::now().to_rfc3339();
        (StatusCode::OK, Json(serde_json::json!(d)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Declaration not found." })),
        )
    }
}

pub async fn verify_declaration(
    State(store): State<TaxDeclarationsStore>,
    Path(id): Path<String>,
    Json(dto): Json<VerifyTaxDeclarationDto>,
) -> impl IntoResponse {
    let mut list = store.declarations.write().await;
    if let Some(d) = list.iter_mut().find(|d| d.id == id) {
        d.status = "Verified".into();
        let verified = dto.verified_amount.unwrap_or(d.total_declared);
        d.total_verified = (verified * 100.0).round() / 100.0;
        d.verification_notes = dto.verification_notes;
        d.rejection_reason = None;
        d.verified_at = Some(Utc::now().to_rfc3339());
        d.updated_at = Utc::now().to_rfc3339();
        (StatusCode::OK, Json(serde_json::json!(d)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Declaration not found." })),
        )
    }
}

pub async fn reject_declaration(
    State(store): State<TaxDeclarationsStore>,
    Path(id): Path<String>,
    Json(dto): Json<RejectTaxDeclarationDto>,
) -> impl IntoResponse {
    let mut list = store.declarations.write().await;
    if let Some(d) = list.iter_mut().find(|d| d.id == id) {
        if d.status == "Verified" {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Verified declarations cannot be rejected."
                })),
            );
        }
        d.status = "Rejected".into();
        d.rejection_reason = Some(dto.reason);
        d.updated_at = Utc::now().to_rfc3339();
        (StatusCode::OK, Json(serde_json::json!(d)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Declaration not found." })),
        )
    }
}

pub async fn delete_declaration(
    State(store): State<TaxDeclarationsStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut list = store.declarations.write().await;
    if let Some(pos) = list.iter().position(|d| d.id == id) {
        if list[pos].status == "Verified" {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Cannot delete verified declarations."
                })),
            );
        }
        list.remove(pos);
        (StatusCode::OK, Json(serde_json::json!({ "success": true })))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Declaration not found." })),
        )
    }
}
