use serde::Serialize;

use crate::model::SourceLocation;

/// A `(struct_type, field)` pair — used by `offsetof()` and `G_STRUCT_OFFSET`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructField {
    pub struct_type: String,
    pub field: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OffsetOfExpression {
    pub struct_field: StructField,
    pub location: SourceLocation,
}
