use serde::Serialize;

use crate::model::{Expression, SourceLocation};

#[derive(Debug, Clone, Serialize)]
pub struct ReturnStatement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Expression>,
    pub location: SourceLocation,
}
