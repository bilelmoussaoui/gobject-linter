use serde::Serialize;

use crate::model::{FieldAccessOp, SourceLocation, expression::Expression};

#[derive(Debug, Clone, Serialize)]
pub struct FieldAccessExpression {
    pub base: Box<Expression>,
    pub operator: FieldAccessOp,
    pub field: String,
    pub location: SourceLocation,
}
