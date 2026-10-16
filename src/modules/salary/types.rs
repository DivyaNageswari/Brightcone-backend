use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SalaryBreakdown {
    pub gross_monthly_rupees: f64,
    pub basic_monthly_rupees: f64,
    pub hra_monthly_rupees: f64,
    pub special_allowance_rupees: f64,
    pub employee_pf_rupees: f64,
    pub employer_pf_rupees: f64,
    pub professional_tax_rupees: f64,
    pub estimated_tds_rupees: f64,
    pub net_take_home_rupees: f64,
    pub annual_ctc_rupees: f64,
    pub gross_monthly_paise: i64,
    pub basic_monthly_paise: i64,
    pub hra_monthly_paise: i64,
    pub special_allowance_paise: i64,
    pub employee_pf_paise: i64,
    pub employer_pf_paise: i64,
    pub professional_tax_paise: i64,
    pub estimated_tds_paise: i64,
    pub net_take_home_paise: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SalaryStructure {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub department: String,
    pub effective_from: String,
    pub gross_monthly: f64,
    pub basic: f64,
    pub hra: f64,
    pub special_allowance: f64,
    pub employee_pf: f64,
    pub professional_tax: f64,
    pub estimated_tds: f64,
    pub net_take_home: f64,
    pub annual_ctc: f64,
    pub pf_opt_out: bool,
    pub tax_regime: String,
    pub is_active: bool,
    pub superseded_by: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SalaryRevision {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub current_gross: f64,
    pub proposed_gross: f64,
    pub increment_percent: f64,
    pub effective_date: String,
    pub reason: String,
    pub notes: String,
    pub status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncrementHistoryRecord {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub previous_gross: f64,
    pub revised_gross: f64,
    pub increment_amount: f64,
    pub increment_percentage: f64,
    pub effective_date: String,
    pub change_type: String,
    pub approved_by: String,
    pub approved_at: String,
    pub notes: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CalculateSalaryDto {
    pub gross_monthly: f64,
    pub pf_capped: Option<bool>,
    pub tax_regime: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateSalaryStructureDto {
    pub employee_id: String,
    pub employee_name: String,
    pub department: String,
    pub gross_monthly: f64,
    pub pf_opt_out: Option<bool>,
    pub tax_regime: Option<String>,
    pub effective_from: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProposeRevisionDto {
    pub employee_id: String,
    pub employee_name: String,
    pub current_gross: f64,
    pub proposed_gross: f64,
    pub effective_date: String,
    pub reason: String,
    pub notes: String,
}
