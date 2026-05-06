use serde::Serialize;

use crate::model::{Expression, SourceLocation, UpdateOp};

#[derive(Debug, Clone, Serialize)]
pub struct UpdateExpression {
    pub operator: UpdateOp,
    pub operand: Box<Expression>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_prefix: bool, // true for ++x, false for x++
    pub location: SourceLocation,
}
