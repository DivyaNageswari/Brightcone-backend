use super::{Computation, FieldDefinition, FieldKind};
const STATUSES: &[&str] = &[
    "active",
    "inactive",
    "pending",
    "approved",
    "rejected",
    "draft",
    "completed",
    "cancelled",
    "in_progress",
    "open",
    "closed",
    "on_leave",
    "terminated",
];
const RELATIONSHIPS: &[&str] = &["spouse", "child", "parent", "sibling", "partner", "other"];
const LEAVE_TYPES: &[&str] = &[
    "annual",
    "sick",
    "personal",
    "unpaid",
    "parental",
    "bereavement",
    "other",
];
const CURRENCIES: &[&str] = &["INR", "USD", "EUR", "GBP", "CAD", "AUD"];
const FIELD_STATUSES: FieldKind = FieldKind::Enum(STATUSES);

const fn field(
    key: &'static str,
    kind: FieldKind,
    required: bool,
    unique: bool,
    max_length: usize,
) -> FieldDefinition {
    FieldDefinition {
        key,
        kind,
        required,
        unique,
        max_length,
        computed: None,
    }
}

const fn computed(key: &'static str, kind: FieldKind, computation: Computation) -> FieldDefinition {
    FieldDefinition {
        key,
        kind,
        required: false,
        unique: false,
        max_length: 320,
        computed: Some(computation),
    }
}

const EMPLOYEE_FIELDS: &[FieldDefinition] = &[
    field("employee_number", FieldKind::String, true, true, 64),
    field("first_name", FieldKind::String, true, false, 100),
    field("last_name", FieldKind::String, true, false, 100),
    field("work_email", FieldKind::Email, true, true, 254),
    field("department_id", FieldKind::String, false, false, 64),
    field("manager_id", FieldKind::String, false, false, 64),
    field("job_title_id", FieldKind::String, false, false, 64),
    field("location_id", FieldKind::String, false, false, 64),
    field(
        "employment_type",
        FieldKind::Enum(&["full_time", "part_time", "contract", "intern"]),
        false,
        false,
        24,
    ),
    field("status", FIELD_STATUSES, true, false, 24),
    field("start_date", FieldKind::Date, true, false, 10),
    field("end_date", FieldKind::Date, false, false, 10),
    field("phone", FieldKind::String, false, false, 32),
    field("metadata", FieldKind::Object, false, false, 16_384),
    computed(
        "full_name",
        FieldKind::String,
        Computation::Concatenate {
            fields: &["first_name", "last_name"],
            separator: " ",
        },
    ),
];

const PERSON_CHILD_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, true, false, 64),
    field("name", FieldKind::String, true, false, 160),
    field("email", FieldKind::Email, false, false, 254),
    field("phone", FieldKind::String, false, false, 32),
    field(
        "relationship",
        FieldKind::Enum(RELATIONSHIPS),
        false,
        false,
        24,
    ),
    field("status", FIELD_STATUSES, false, false, 24),
    field("effective_date", FieldKind::Date, false, false, 10),
    field("metadata", FieldKind::Object, false, false, 16_384),
];

const ADDRESS_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, true, false, 64),
    field(
        "address_type",
        FieldKind::Enum(&["home", "work", "mailing"]),
        true,
        false,
        16,
    ),
    field("line_1", FieldKind::String, true, false, 200),
    field("line_2", FieldKind::String, false, false, 200),
    field("city", FieldKind::String, true, false, 100),
    field("region", FieldKind::String, false, false, 100),
    field("postal_code", FieldKind::String, false, false, 20),
    field("country_code", FieldKind::String, true, false, 2),
];

const ASSET_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, false, false, 64),
    field("asset_tag", FieldKind::String, true, true, 80),
    field("asset_type", FieldKind::String, true, false, 80),
    field("status", FIELD_STATUSES, true, false, 24),
    field("assigned_date", FieldKind::Date, false, false, 10),
    field("returned_date", FieldKind::Date, false, false, 10),
    field("description", FieldKind::String, false, false, 2_000),
];

const ORG_FIELDS: &[FieldDefinition] = &[
    field("code", FieldKind::String, true, true, 64),
    field("name", FieldKind::String, true, false, 160),
    field("description", FieldKind::String, false, false, 2_000),
    field("parent_id", FieldKind::String, false, false, 64),
    field("status", FIELD_STATUSES, true, false, 24),
    field("metadata", FieldKind::Object, false, false, 16_384),
];

const LEAVE_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, true, false, 64),
    field("leave_type", FieldKind::Enum(LEAVE_TYPES), true, false, 24),
    field("starts_on", FieldKind::Date, true, false, 10),
    field("ends_on", FieldKind::Date, true, false, 10),
    computed(
        "days",
        FieldKind::Integer,
        Computation::InclusiveDateSpan {
            start: "starts_on",
            end: "ends_on",
        },
    ),
    field("status", FIELD_STATUSES, true, false, 24),
    field("reason", FieldKind::String, false, false, 2_000),
    field("metadata", FieldKind::Object, false, false, 16_384),
];

const SCHEDULE_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, true, false, 64),
    field("starts_at", FieldKind::String, true, false, 40),
    field("ends_at", FieldKind::String, true, false, 40),
    field("status", FIELD_STATUSES, true, false, 24),
    field("reason", FieldKind::String, false, false, 2_000),
    field("metadata", FieldKind::Object, false, false, 16_384),
];

const RECRUITING_FIELDS: &[FieldDefinition] = &[
    field("name", FieldKind::String, false, false, 160),
    field("first_name", FieldKind::String, false, false, 100),
    field("last_name", FieldKind::String, false, false, 100),
    field("email", FieldKind::Email, false, false, 254),
    field("title", FieldKind::String, false, false, 160),
    field("requisition_code", FieldKind::String, false, true, 64),
    field("job_id", FieldKind::String, false, false, 64),
    field(
        "stage",
        FieldKind::Enum(&[
            "applied",
            "screening",
            "interview",
            "offer",
            "hired",
            "rejected",
        ]),
        false,
        false,
        24,
    ),
    field("status", FIELD_STATUSES, false, false, 24),
    field("effective_date", FieldKind::Date, false, false, 10),
    field("description", FieldKind::String, false, false, 4_000),
    field("metadata", FieldKind::Object, false, false, 16_384),
    computed(
        "full_name",
        FieldKind::String,
        Computation::Concatenate {
            fields: &["first_name", "last_name"],
            separator: " ",
        },
    ),
];

const FINANCE_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, false, false, 64),
    field("name", FieldKind::String, false, false, 160),
    field("amount", FieldKind::Decimal, false, false, 0),
    field("tax_amount", FieldKind::Decimal, false, false, 0),
    field("currency", FieldKind::Enum(CURRENCIES), false, false, 3),
    field("effective_date", FieldKind::Date, false, false, 10),
    field("status", FIELD_STATUSES, false, false, 24),
    field("description", FieldKind::String, false, false, 2_000),
    field("metadata", FieldKind::Object, false, false, 16_384),
    computed(
        "total_amount",
        FieldKind::Decimal,
        Computation::Sum(&["amount", "tax_amount"]),
    ),
];

const LEARNING_FIELDS: &[FieldDefinition] = &[
    field("employee_id", FieldKind::String, true, false, 64),
    field("course_code", FieldKind::String, false, true, 64),
    field("course_name", FieldKind::String, false, false, 160),
    field("status", FIELD_STATUSES, true, false, 24),
    field("due_date", FieldKind::Date, false, false, 10),
    field("completed_date", FieldKind::Date, false, false, 10),
    field("score", FieldKind::Decimal, false, false, 0),
    field("metadata", FieldKind::Object, false, false, 16_384),
];

const GENERAL_FIELDS: &[FieldDefinition] = &[
    field("name", FieldKind::String, true, false, 160),
    field("code", FieldKind::String, false, true, 64),
    field("status", FIELD_STATUSES, true, false, 24),
    field("description", FieldKind::String, false, false, 4_000),
    field("employee_id", FieldKind::String, false, false, 64),
    field("effective_date", FieldKind::Date, false, false, 10),
    field("metadata", FieldKind::Object, false, false, 16_384),
];

#[path = "schemas.rs"]
mod schemas;
pub use schemas::{find_field, schema_for, RecordSchema};
