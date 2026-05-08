use serde::Serialize;

use crate::model::{AssignmentOp, Expression, SourceLocation};

#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub lhs: Box<Expression>, // Can be Identifier or FieldAccess
    pub operator: AssignmentOp,
    pub rhs: Box<Expression>,
    pub location: SourceLocation,
}

impl Assignment {
    pub fn lhs_as_text<'a>(&self, source: &'a [u8]) -> &'a str {
        self.lhs.location().as_str(source).unwrap_or("")
    }
}
