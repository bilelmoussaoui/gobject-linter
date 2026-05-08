use serde::Serialize;

use crate::{
    model::{SourceLocation, expression::Expression},
    operators::FieldAccessOp,
};

#[derive(Debug, Clone, Serialize)]
pub struct FieldAccessExpression {
    pub base: Box<Expression>,
    pub operator: FieldAccessOp,
    pub field: String,
    pub location: SourceLocation,
}
