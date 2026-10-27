use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaxDeclaration {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub department: String,
    pub financial_year: String,
    pub tax_regime: String,
    pub status: String,
    pub section_80c: f64,
    pub section_80d: f64,
    pub section_80ccd_nps: f64,
    pub section_24_home_loan: f64,
    pub hra_annual_rent: f64,
    pub hra_landlord_pan: Option<String>,
    pub hra_landlord_name: Option<String>,
    pub hra_rental_address: Option<String>,
    pub section_80e_education_loan: f64,
    pub section_80g_donations: f64,
    pub other_exemptions: f64,
    pub total_declared: f64,
    pub total_verified: f64,
    pub proof_attachments_count: usize,
    pub rejection_reason: Option<String>,
    pub verification_notes: Option<String>,
    pub submitted_at: Option<String>,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateTaxDeclarationDto {
    pub employee_id: Option<String>,
    pub employee_name: Option<String>,
    pub department: Option<String>,
    pub financial_year: Option<String>,
    pub tax_regime: Option<String>,
    pub section_80c: Option<f64>,
    pub section_80d: Option<f64>,
    pub section_80ccd_nps: Option<f64>,
    pub section_24_home_loan: Option<f64>,
    pub hra_annual_rent: Option<f64>,
    pub hra_landlord_pan: Option<String>,
    pub hra_landlord_name: Option<String>,
    pub hra_rental_address: Option<String>,
    pub section_80e_education_loan: Option<f64>,
    pub section_80g_donations: Option<f64>,
    pub other_exemptions: Option<f64>,
    pub proof_attachments_count: Option<usize>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateTaxDeclarationDto {
    pub tax_regime: Option<String>,
    pub section_80c: Option<f64>,
    pub section_80d: Option<f64>,
    pub section_80ccd_nps: Option<f64>,
    pub section_24_home_loan: Option<f64>,
    pub hra_annual_rent: Option<f64>,
    pub hra_landlord_pan: Option<String>,
    pub hra_landlord_name: Option<String>,
    pub hra_rental_address: Option<String>,
    pub section_80e_education_loan: Option<f64>,
    pub section_80g_donations: Option<f64>,
    pub other_exemptions: Option<f64>,
    pub proof_attachments_count: Option<usize>,
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VerifyTaxDeclarationDto {
    pub verified_amount: Option<f64>,
    pub verification_notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RejectTaxDeclarationDto {
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TaxDeclarationSummary {
    pub total_declarations: usize,
    pub pending_verification: usize,
    pub verified_count: usize,
    pub rejected_count: usize,
    pub draft_count: usize,
    pub total_declared_amount: f64,
    pub total_verified_amount: f64,
    pub old_regime_count: usize,
    pub new_regime_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeclarationFilterQuery {
    pub employee_id: Option<String>,
    pub status: Option<String>,
    pub regime: Option<String>,
}
