use serde::Serialize;

use crate::model::{SourceLocation, expression::Expression, statement::Statement};

#[derive(Debug, Clone, Serialize)]
pub struct WhileStatement {
    /// Condition expression
    pub condition: Box<Expression>,
    /// Loop body
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoWhileStatement {
    /// Loop body
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<Statement>,
    /// Condition expression
    pub condition: Box<Expression>,
    pub location: SourceLocation,
}
