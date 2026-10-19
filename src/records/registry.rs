//! Static, code-owned schemas for the generic HR record API.

pub const REGISTRY_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    String,
    Integer,
    Decimal,
    Boolean,
    Date,
    Email,
    Enum(&'static [&'static str]),
    Object,
}

#[derive(Debug, Clone, Copy)]
pub enum Computation {
    Concatenate {
        fields: &'static [&'static str],
        separator: &'static str,
    },
    Sum(&'static [&'static str]),
    InclusiveDateSpan {
        start: &'static str,
        end: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct FieldDefinition {
    pub key: &'static str,
    pub kind: FieldKind,
    pub required: bool,
    pub unique: bool,
    pub max_length: usize,
    pub computed: Option<Computation>,
}

mod fields;
pub use fields::RecordSchema;
pub use fields::{find_field, schema_for};
