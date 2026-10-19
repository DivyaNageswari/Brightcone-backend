use loco_rs::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    authz::AuthUser,
    records::{
        self,
        registry::{self, Computation, FieldKind},
    },
};

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldSchema {
    pub key: String,
    pub kind: String,
    pub required: bool,
    pub unique: bool,
    pub max_length: usize,
    pub allowed_values: Vec<String>,
    pub computed: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordTypeSchema {
    pub key: String,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordCatalog {
    pub version: u16,
    pub record_types: Vec<RecordTypeSchema>,
}

fn field_kind(kind: FieldKind) -> (String, Vec<String>) {
    match kind {
        FieldKind::String => ("string".to_owned(), Vec::new()),
        FieldKind::Integer => ("integer".to_owned(), Vec::new()),
        FieldKind::Decimal => ("decimal".to_owned(), Vec::new()),
        FieldKind::Boolean => ("boolean".to_owned(), Vec::new()),
        FieldKind::Date => ("date".to_owned(), Vec::new()),
        FieldKind::Email => ("email".to_owned(), Vec::new()),
        FieldKind::Object => ("object".to_owned(), Vec::new()),
        FieldKind::Enum(values) => (
            "enum".to_owned(),
            values.iter().map(|value| (*value).to_owned()).collect(),
        ),
    }
}

fn computed_name(computation: Computation) -> String {
    match computation {
        Computation::Concatenate { fields, separator } => {
            format!("concat:{}:{}", fields.join(","), separator)
        }
        Computation::Sum(fields) => format!("sum:{}", fields.join(",")),
        Computation::InclusiveDateSpan { start, end } => {
            format!("inclusive_date_span:{start},{end}")
        }
    }
}

fn catalog() -> RecordCatalog {
    let record_types = records::RECORD_TYPES
        .iter()
        .filter_map(|key| {
            let schema = registry::schema_for(key)?;
            let fields = schema
                .fields
                .iter()
                .map(|definition| {
                    let (kind, allowed_values) = field_kind(definition.kind);
                    FieldSchema {
                        key: definition.key.to_owned(),
                        kind,
                        required: schema.required.contains(&definition.key) || definition.required,
                        unique: definition.unique,
                        max_length: definition.max_length,
                        allowed_values,
                        computed: definition.computed.map(computed_name),
                    }
                })
                .collect();
            Some(RecordTypeSchema {
                key: (*key).to_owned(),
                fields,
            })
        })
        .collect();
    RecordCatalog {
        version: registry::REGISTRY_VERSION,
        record_types,
    }
}

#[utoipa::path(get, path = "/api/record-types", security(("bearer" = [])), responses((status = 200, body = RecordCatalog)))]
#[debug_handler]
pub async fn list(auth: AuthUser, State(_ctx): State<AppContext>) -> Result<Response> {
    auth.require("records:read")?;
    format::json(catalog())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/record-types")
        .add("/", get(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_exposes_complete_field_contracts() {
        let result = catalog();
        assert_eq!(result.version, 1);
        assert_eq!(result.record_types.len(), 50);
        let employee = result
            .record_types
            .iter()
            .find(|kind| kind.key == "employee")
            .expect("employee schema");
        assert!(employee
            .fields
            .iter()
            .any(|field| field.key == "work_email" && field.unique && field.required));
        assert!(employee
            .fields
            .iter()
            .any(|field| field.key == "full_name" && field.computed.is_some()));
    }
}
