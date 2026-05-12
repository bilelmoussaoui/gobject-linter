use serde::Serialize;

use crate::model::{Expression, SourceLocation, TypeInfo};

#[derive(Debug, Clone, Serialize)]
pub struct VariableDecl {
    pub type_info: TypeInfo,
    pub name: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_static: bool,
    /// Location of the variable name in the source
    #[serde(skip)]
    pub name_location: SourceLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initializer: Option<Expression>,
    /// Array size expression for array declarators (e.g., [N_PROPS])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_size: Option<Expression>,
    pub location: SourceLocation,
}

impl VariableDecl {
    /// Check if this is a simple identifier (not a field access like
    /// obj->field)
    pub fn is_simple_identifier(&self) -> bool {
        !self.name.contains("->") && !self.name.contains('.')
    }
}
