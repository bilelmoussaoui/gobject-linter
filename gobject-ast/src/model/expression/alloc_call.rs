use serde::Serialize;

use crate::model::{Expression, SourceLocation, TypeInfo};

/// GLib allocation call: g_new(Type, n), g_renew(Type *, ptr, n), etc.
/// These take a type as the first argument, not an expression.
#[derive(Debug, Clone, Serialize)]
pub struct AllocCallExpression {
    /// The allocation function (g_new, g_new0, g_renew, g_slice_new, etc.)
    pub function: Box<Expression>,

    /// The type being allocated (parsed into TypeInfo)
    pub allocated_type: TypeInfo,

    /// The remaining arguments (e.g., count, existing pointer, etc.)
    pub arguments: Vec<Box<Expression>>,

    pub location: SourceLocation,
}
