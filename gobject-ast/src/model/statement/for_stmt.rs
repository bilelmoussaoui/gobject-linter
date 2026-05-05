use serde::{Deserialize, Serialize};

use crate::model::{SourceLocation, expression::Expression, statement::Statement};

/// The initializer clause of a `for` statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForInit {
    /// Expression initializer: `for (i = 0; ...)`
    Expr(Box<Expression>),
    /// Declaration initializer: `for (int i = 0; ...)` or `for (GList *l =
    /// list; ...)`
    Decl(Box<super::VariableDecl>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForStatement {
    pub initializer: Option<ForInit>,
    /// Condition expression
    pub condition: Option<Box<Expression>>,
    /// Update expression
    pub update: Option<Box<Expression>>,
    /// Loop body (can be single statement or compound)
    pub body: Vec<Statement>,
    pub location: SourceLocation,
}
