use serde::Serialize;

use crate::model::{BinaryExpression, Expression, SourceLocation, Statement};

#[derive(Debug, Clone, Serialize)]
pub struct IfStatement {
    pub condition: Expression,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub then_body: Vec<Statement>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub then_has_braces: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub else_body: Option<Vec<Statement>>,
    pub location: SourceLocation,
}

impl IfStatement {
    /// Extract variable from NULL check patterns: (ptr != NULL), (NULL != ptr),
    /// (ptr)
    pub fn extract_null_check_variable<'a>(&self, source: &'a [u8]) -> Option<&'a str> {
        match &self.condition {
            Expression::Binary(bin) if bin.is_null_check() => bin.extract_compared_variable(source),
            expr => expr.extract_variable_name(source),
        }
    }

    /// Extract variable from non-zero check patterns: (id > 0), (id != 0),
    /// (id), (self->id)
    pub fn extract_nonzero_check_variable<'a>(&self, source: &'a [u8]) -> Option<&'a str> {
        match &self.condition {
            Expression::Binary(bin) => {
                if is_nonzero_comparison(bin) {
                    bin.extract_compared_variable(source)
                } else {
                    None
                }
            }
            expr => expr.extract_variable_name(source),
        }
    }

    /// Check if condition is a simple truthiness test (just a variable)
    pub fn is_truthiness_check(&self) -> bool {
        matches!(&self.condition, Expression::Identifier(_))
    }

    /// Check if then body has exactly one statement
    pub fn has_single_statement(&self) -> bool {
        self.then_body.len() == 1
    }

    /// Check if else branch exists
    pub fn has_else(&self) -> bool {
        self.else_body.is_some()
    }
}

/// Helper to check if binary expression is a non-zero comparison
fn is_nonzero_comparison(bin: &BinaryExpression) -> bool {
    match bin.operator.as_str() {
        "!=" | ">" | "<" => {
            // Check if comparing to 0
            bin.left.is_zero() || bin.right.is_zero()
        }
        _ => false,
    }
}
