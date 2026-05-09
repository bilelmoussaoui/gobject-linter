use serde::Serialize;

use crate::model::{Expression, SourceLocation, Statement, VariableDecl};

/// The initializer clause of a `for` statement.
#[derive(Debug, Clone, Serialize)]
pub enum ForInit {
    /// Expression initializer: `for (i = 0; ...)`
    Expr(Box<Expression>),
    /// Declaration initializer: `for (int i = 0; ...)` or `for (GList *l =
    /// list; ...)`
    Decl(Box<VariableDecl>),
}

#[derive(Debug, Clone, Serialize)]
pub struct ForStatement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initializer: Option<ForInit>,
    /// Condition expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Box<Expression>>,
    /// Update expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<Box<Expression>>,
    /// Loop body (can be single statement or compound)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}
