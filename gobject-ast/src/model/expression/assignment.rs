use serde::Serialize;

use crate::model::{AssignmentOp, Expression, SourceLocation, UnaryOp};

#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub lhs: Box<Expression>, // Can be Identifier or FieldAccess
    pub operator: AssignmentOp,
    pub rhs: Box<Expression>,
    pub location: SourceLocation,
}

impl Assignment {
    /// Get a string representation of the LHS (identifier name or field access
    /// text) Handles identifiers, field access, and dereference expressions
    pub fn lhs_as_text(&self) -> String {
        match &*self.lhs {
            Expression::Identifier(id) => id.name.clone(),
            Expression::FieldAccess(field) => field.text(),
            Expression::Unary(unary) if unary.operator == UnaryOp::Dereference => {
                // *var -> extract var name
                if let Expression::Identifier(id) = &*unary.operand {
                    format!("*{}", id.name)
                } else {
                    unary.operand.extract_variable_name().unwrap_or_default()
                }
            }
            _ => self.lhs.extract_variable_name().unwrap_or_default(),
        }
    }
}
