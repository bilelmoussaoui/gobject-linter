use serde::Serialize;

use crate::model::{Expression, SourceLocation};

#[derive(Debug, Clone, Serialize)]
pub struct ConditionalExpression {
    pub condition: Box<Expression>,
    pub then_expr: Box<Expression>,
    pub else_expr: Box<Expression>,
    pub location: SourceLocation,
}
