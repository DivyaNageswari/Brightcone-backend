pub mod registry;

use chrono::NaiveDate;
use registry::{Computation, FieldDefinition, FieldKind, RecordSchema};
use serde_json::{Map, Number, Value};

pub const RECORD_TYPES: &[&str] = &[
    "employee",
    "employee_document",
    "employee_emergency_contact",
    "employee_address",
    "employee_bank_account",
    "employee_dependent",
    "employee_asset",
    "employee_note",
    "employee_onboarding",
    "employee_offboarding",
    "department",
    "location",
    "job_title",
    "employment_type",
    "leave_request",
    "leave_balance",
    "leave_policy",
    "holiday",
    "attendance_entry",
    "attendance_adjustment",
    "timesheet",
    "shift",
    "shift_swap",
    "overtime_request",
    "candidate",
    "job_requisition",
    "application",
    "interview",
    "offer",
    "recruitment_stage",
    "payroll_run",
    "payslip",
    "payroll_adjustment",
    "tax_declaration",
    "expense_claim",
    "expense_category",
    "reimbursement",
    "benefit",
    "training_course",
    "training_enrollment",
    "certification",
    "performance_review",
    "goal",
    "promotion",
    "recognition",
    "announcement",
    "survey",
    "access_request",
    "compliance_item",
    "custom_record",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueClaim {
    pub field_key: String,
    pub normalized_value: String,
}

pub fn is_registered(record_type: &str) -> bool {
    registry::schema_for(record_type).is_some()
}

pub fn validate_record(record_type: &str, data: &Value) -> Result<Value, String> {
    let schema =
        registry::schema_for(record_type).ok_or_else(|| "Unknown record type.".to_owned())?;
    let mut object = data
        .as_object()
        .cloned()
        .ok_or_else(|| "Record data must be a JSON object.".to_owned())?;
    if object.len() > 100 {
        return Err("Record has too many fields.".to_owned());
    }
    if data.to_string().len() > 512_000 {
        return Err("Record exceeds the 512 KB data limit.".to_owned());
    }

    for (key, value) in &object {
        let definition = registry::find_field(schema, key)
            .ok_or_else(|| format!("Field '{key}' is not allowed for '{record_type}'."))?;
        if definition.computed.is_some() {
            return Err(format!("Field '{key}' is computed and cannot be supplied."));
        }
        validate_value(definition, value)?;
    }
    for key in schema.required {
        if !object.contains_key(*key) || object.get(*key).is_some_and(Value::is_null) {
            return Err(format!("Field '{key}' is required."));
        }
    }
    apply_computed(schema, &mut object)?;
    Ok(Value::Object(object))
}

fn validate_value(definition: &FieldDefinition, value: &Value) -> Result<(), String> {
    if value.is_null() {
        return if definition.required {
            Err(format!("Field '{}' is required.", definition.key))
        } else {
            Ok(())
        };
    }
    if matches!(definition.kind, FieldKind::Object)
        && value.to_string().len() > definition.max_length
    {
        return Err(format!(
            "Field '{}' exceeds its size limit.",
            definition.key
        ));
    }
    let valid = match definition.kind {
        FieldKind::String => value
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() <= definition.max_length),
        FieldKind::Integer => value.as_i64().is_some(),
        FieldKind::Decimal => value.as_f64().is_some_and(f64::is_finite),
        FieldKind::Boolean => value.is_boolean(),
        FieldKind::Date => value
            .as_str()
            .is_some_and(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()),
        FieldKind::Email => value.as_str().is_some_and(valid_email),
        FieldKind::Enum(values) => value.as_str().is_some_and(|s| values.contains(&s)),
        FieldKind::Object => value.is_object(),
    };
    if valid {
        Ok(())
    } else {
        Err(format!("Field '{}' has an invalid value.", definition.key))
    }
}

fn valid_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let Some(local) = parts.next() else {
        return false;
    };
    let Some(domain) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

fn apply_computed(schema: &RecordSchema, object: &mut Map<String, Value>) -> Result<(), String> {
    for definition in schema
        .fields
        .iter()
        .filter(|field| field.computed.is_some())
    {
        let value = match definition.computed.expect("filtered computed field") {
            Computation::Concatenate { fields, separator } => {
                let parts = fields
                    .iter()
                    .filter_map(|key| object.get(*key)?.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .collect::<Vec<_>>();
                if parts.is_empty() {
                    None
                } else {
                    Some(Value::String(parts.join(separator)))
                }
            }
            Computation::Sum(fields) => {
                let values = fields
                    .iter()
                    .filter_map(|key| object.get(*key).and_then(Value::as_f64))
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    None
                } else {
                    let total: f64 = values.iter().sum();
                    Some(Number::from_f64(total).map(Value::Number).ok_or_else(|| {
                        format!("Computed field '{}' overflowed.", definition.key)
                    })?)
                }
            }
            Computation::InclusiveDateSpan { start, end } => {
                let start = object.get(start).and_then(Value::as_str);
                let end = object.get(end).and_then(Value::as_str);
                match (start, end) {
                    (Some(start), Some(end)) => {
                        let start = NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| {
                            format!(
                                "Computed field '{}' has an invalid start date.",
                                definition.key
                            )
                        })?;
                        let end = NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|_| {
                            format!(
                                "Computed field '{}' has an invalid end date.",
                                definition.key
                            )
                        })?;
                        if end < start {
                            return Err(format!(
                                "Computed field '{}' has an end before its start.",
                                definition.key
                            ));
                        }
                        Some(Value::Number(Number::from((end - start).num_days() + 1)))
                    }
                    _ => None,
                }
            }
        };
        if let Some(value) = value {
            validate_value(definition, &value)?;
            object.insert(definition.key.to_owned(), value);
        }
    }
    Ok(())
}

pub fn unique_claims(record_type: &str, data: &Value) -> Vec<UniqueClaim> {
    let Some(schema) = registry::schema_for(record_type) else {
        return Vec::new();
    };
    schema
        .fields
        .iter()
        .filter(|field| field.unique)
        .filter_map(|field| {
            let value = data.get(field.key)?;
            if value.is_null() {
                return None;
            }
            let normalized_value = match field.kind {
                FieldKind::String | FieldKind::Email => {
                    value.as_str().unwrap_or_default().trim().to_lowercase()
                }
                _ => value.to_string(),
            };
            Some(UniqueClaim {
                field_key: field.key.to_owned(),
                normalized_value,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalog_contains_fifty_registered_schemas() {
        assert_eq!(RECORD_TYPES.len(), 50);
        assert!(RECORD_TYPES.iter().all(|key| is_registered(key)));
        assert!(!is_registered("arbitrary_table"));
    }

    #[test]
    fn employee_validation_checks_required_types_and_computes_name() {
        let data = validate_record(
            "employee",
            &json!({
                "employee_number": "E-100", "first_name": "Ada", "last_name": "Lovelace",
                "work_email": "ada@brightcone.ai", "status": "active", "start_date": "2026-01-01"
            }),
        )
        .expect("valid employee");
        assert_eq!(data["full_name"], "Ada Lovelace");
        assert!(validate_record("employee", &json!({"first_name": "Ada"})).is_err());
        assert!(validate_record(
            "employee",
            &json!({
                "employee_number": "E-101", "first_name": "Ada", "last_name": "L",
                "work_email": "bad", "status": "active", "start_date": "2026-01-01"
            })
        )
        .is_err());
    }

    #[test]
    fn schemas_reject_unknown_fields_and_unsafe_credentials() {
        assert!(validate_record("employee", &json!({"password": "x"})).is_err());
        assert!(
            validate_record("employee", &json!({"employee_number": "1", "extra": true})).is_err()
        );
    }

    #[test]
    fn unique_claims_normalize_case_and_computed_sum_is_deterministic() {
        let employee = json!({"work_email":"ADA@BRIGHTCONE.AI"});
        assert_eq!(
            unique_claims("employee", &employee)[0].normalized_value,
            "ada@brightcone.ai"
        );
        let expense = validate_record(
            "expense_claim",
            &json!({"amount":100.0,"tax_amount":18.0,"currency":"INR","status":"pending"}),
        )
        .expect("valid expense");
        assert_eq!(expense["total_amount"], 118.0);
        let leave = validate_record(
            "leave_request",
            &json!({
                "employee_id":"E-1", "leave_type":"annual", "starts_on":"2026-10-19",
                "ends_on":"2026-10-20", "status":"pending"
            }),
        )
        .expect("valid leave");
        assert_eq!(leave["days"], 2);
        assert!(validate_record(
            "leave_request",
            &json!({
                "employee_id":"E-1", "leave_type":"annual", "starts_on":"2026-10-20",
                "ends_on":"2026-10-19", "status":"pending"
            })
        )
        .is_err());
    }
}
