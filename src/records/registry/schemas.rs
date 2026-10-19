use super::*;
#[derive(Debug)]
pub struct RecordSchema {
    pub fields: &'static [FieldDefinition],
    pub required: &'static [&'static str],
}

const EMPLOYEE_SCHEMA: RecordSchema = RecordSchema {
    fields: EMPLOYEE_FIELDS,
    required: &[
        "employee_number",
        "first_name",
        "last_name",
        "work_email",
        "status",
        "start_date",
    ],
};
const PEOPLE_SCHEMA: RecordSchema = RecordSchema {
    fields: PERSON_CHILD_FIELDS,
    required: &["employee_id", "name"],
};
const ADDRESS_SCHEMA: RecordSchema = RecordSchema {
    fields: ADDRESS_FIELDS,
    required: &[
        "employee_id",
        "address_type",
        "line_1",
        "city",
        "country_code",
    ],
};
const ASSET_SCHEMA: RecordSchema = RecordSchema {
    fields: ASSET_FIELDS,
    required: &["asset_tag", "asset_type", "status"],
};
const ORG_SCHEMA: RecordSchema = RecordSchema {
    fields: ORG_FIELDS,
    required: &["code", "name", "status"],
};
const LEAVE_SCHEMA: RecordSchema = RecordSchema {
    fields: LEAVE_FIELDS,
    required: &[
        "employee_id",
        "leave_type",
        "starts_on",
        "ends_on",
        "status",
    ],
};
const SCHEDULE_SCHEMA: RecordSchema = RecordSchema {
    fields: SCHEDULE_FIELDS,
    required: &["employee_id", "starts_at", "ends_at", "status"],
};
const RECRUITING_SCHEMA: RecordSchema = RecordSchema {
    fields: RECRUITING_FIELDS,
    required: &[],
};
const FINANCE_SCHEMA: RecordSchema = RecordSchema {
    fields: FINANCE_FIELDS,
    required: &["amount", "currency", "status"],
};
const LEARNING_SCHEMA: RecordSchema = RecordSchema {
    fields: LEARNING_FIELDS,
    required: &["employee_id", "status"],
};
const GENERAL_SCHEMA: RecordSchema = RecordSchema {
    fields: GENERAL_FIELDS,
    required: &["name", "status"],
};

pub fn schema_for(record_type: &str) -> Option<&'static RecordSchema> {
    let schema = match record_type {
        "employee" => &EMPLOYEE_SCHEMA,
        "employee_document"
        | "employee_emergency_contact"
        | "employee_dependent"
        | "employee_note"
        | "employee_onboarding"
        | "employee_offboarding" => &PEOPLE_SCHEMA,
        "employee_address" => &ADDRESS_SCHEMA,
        "employee_bank_account" => &PEOPLE_SCHEMA,
        "employee_asset" => &ASSET_SCHEMA,
        "department" | "location" | "job_title" | "employment_type" => &ORG_SCHEMA,
        "leave_request" | "leave_balance" | "leave_policy" | "holiday" => &LEAVE_SCHEMA,
        "attendance_entry"
        | "attendance_adjustment"
        | "timesheet"
        | "shift"
        | "shift_swap"
        | "overtime_request" => &SCHEDULE_SCHEMA,
        "candidate" | "job_requisition" | "application" | "interview" | "offer"
        | "recruitment_stage" => &RECRUITING_SCHEMA,
        "payroll_run" | "payslip" | "payroll_adjustment" | "tax_declaration" | "expense_claim"
        | "expense_category" | "reimbursement" | "benefit" => &FINANCE_SCHEMA,
        "training_course" | "training_enrollment" | "certification" => &LEARNING_SCHEMA,
        "performance_review" | "goal" | "promotion" | "recognition" | "announcement" | "survey"
        | "access_request" | "compliance_item" | "custom_record" => &GENERAL_SCHEMA,
        _ => return None,
    };
    Some(schema)
}

pub fn find_field(schema: &RecordSchema, key: &str) -> Option<&'static FieldDefinition> {
    schema
        .fields
        .iter()
        .find(|definition| definition.key == key)
}
