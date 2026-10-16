use super::types::SalaryBreakdown;

pub fn compute_salary_in_paise(
    gross_rupees: f64,
    pf_capped: bool,
    regime: &str,
) -> SalaryBreakdown {
    let gross_monthly_paise = (gross_rupees * 100.0).round() as i64;
    let basic_monthly_paise = gross_monthly_paise / 2;
    let hra_monthly_paise = basic_monthly_paise / 2;
    let special_allowance_paise = gross_monthly_paise - basic_monthly_paise - hra_monthly_paise;

    let raw_pf_paise = (basic_monthly_paise as f64 * 0.12).round() as i64;
    let employee_pf_paise = if pf_capped && raw_pf_paise > 180_000 {
        180_000
    } else {
        raw_pf_paise
    };
    let employer_pf_paise = employee_pf_paise;

    let professional_tax_paise: i64 = if gross_monthly_paise > 1_500_000 {
        20_000
    } else {
        15_000
    };

    let annual_gross_paise = gross_monthly_paise * 12;
    let std_deduction_paise: i64 = if regime.eq_ignore_ascii_case("new") {
        7_500_000
    } else {
        5_000_000
    };

    let taxable_paise = (annual_gross_paise - std_deduction_paise).max(0);
    let annual_tax_paise: i64 = if regime.eq_ignore_ascii_case("new") {
        if taxable_paise <= 70_000_000 {
            0
        } else {
            let mut tax = 0.0;
            let t = taxable_paise as f64;
            if t > 150_000_000.0 {
                tax += (t - 150_000_000.0) * 0.30;
                tax += 30_000_000.0 * 0.20;
                tax += 20_000_000.0 * 0.15;
                tax += 30_000_000.0 * 0.10;
                tax += 40_000_000.0 * 0.05;
            } else if t > 120_000_000.0 {
                tax += (t - 120_000_000.0) * 0.20;
                tax += 20_000_000.0 * 0.15;
                tax += 30_000_000.0 * 0.10;
                tax += 40_000_000.0 * 0.05;
            } else if t > 100_000_000.0 {
                tax += (t - 100_000_000.0) * 0.15;
                tax += 30_000_000.0 * 0.10;
                tax += 40_000_000.0 * 0.05;
            } else if t > 70_000_000.0 {
                tax += (t - 70_000_000.0) * 0.10;
                tax += 40_000_000.0 * 0.05;
            }
            (tax * 1.04).round() as i64
        }
    } else if taxable_paise <= 50_000_000 {
        0
    } else {
        let t = taxable_paise as f64;
        let mut tax = 0.0;
        if t > 100_000_000.0 {
            tax += (t - 100_000_000.0) * 0.30;
            tax += 50_000_000.0 * 0.20;
            tax += 25_000_000.0 * 0.05;
        } else if t > 50_000_000.0 {
            tax += (t - 50_000_000.0) * 0.20;
            tax += 25_000_000.0 * 0.05;
        }
        (tax * 1.04).round() as i64
    };

    let estimated_tds_paise = annual_tax_paise / 12;
    let net_take_home_paise =
        gross_monthly_paise - employee_pf_paise - professional_tax_paise - estimated_tds_paise;

    SalaryBreakdown {
        gross_monthly_rupees: gross_monthly_paise as f64 / 100.0,
        basic_monthly_rupees: basic_monthly_paise as f64 / 100.0,
        hra_monthly_rupees: hra_monthly_paise as f64 / 100.0,
        special_allowance_rupees: special_allowance_paise as f64 / 100.0,
        employee_pf_rupees: employee_pf_paise as f64 / 100.0,
        employer_pf_rupees: employer_pf_paise as f64 / 100.0,
        professional_tax_rupees: professional_tax_paise as f64 / 100.0,
        estimated_tds_rupees: estimated_tds_paise as f64 / 100.0,
        net_take_home_rupees: net_take_home_paise as f64 / 100.0,
        annual_ctc_rupees: (gross_monthly_paise + employer_pf_paise) as f64 * 12.0 / 100.0,
        gross_monthly_paise,
        basic_monthly_paise,
        hra_monthly_paise,
        special_allowance_paise,
        employee_pf_paise,
        employer_pf_paise,
        professional_tax_paise,
        estimated_tds_paise,
        net_take_home_paise,
    }
}
