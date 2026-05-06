use serde::Serialize;

use crate::model::{SourceLocation, expression::Expression};

#[derive(Debug, Clone, Serialize)]
pub struct InitializerListExpression {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<InitializerItem>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Serialize)]
pub struct InitializerItem {
    /// Optional designator (.field or [index])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designator: Option<Designator>,
    /// The value expression
    pub value: Box<Expression>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Designator {
    /// Field designator: .field_name
    Field(String),
    /// Array/subscript designator: [index]
    Subscript(Box<Expression>),
}
