use serde::Serialize;

use crate::model::{SourceLocation, TypeInfo, expression::Expression};

#[derive(Debug, Clone, Serialize)]
pub struct SizeofExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operand: Option<SizeofOperand>,
    pub text: String, // Full text like "sizeof(int)" or "sizeof x"
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeofOperand {
    Type(TypeInfo),              // sizeof(MyType) or sizeof(struct MyType *)
    Expression(Box<Expression>), // sizeof(expr)
}

impl SizeofExpression {
    /// Get the type if this is sizeof(Type)
    /// Returns Some for both explicit types and simple identifiers (which are
    /// likely types)
    pub fn type_name(&self) -> Option<&str> {
        match &self.operand {
            Some(SizeofOperand::Type(t)) => Some(&t.base_type),
            Some(SizeofOperand::Expression(expr)) => {
                if let Expression::Identifier(id) = expr.as_ref() {
                    Some(&id.name)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    /// Check if this is sizeof of a simple type (not a complex expression)
    pub fn is_sizeof_type(&self) -> bool {
        match &self.operand {
            Some(SizeofOperand::Type(_)) => true,
            // Simple identifier is likely a type
            Some(SizeofOperand::Expression(expr)) => {
                matches!(expr.as_ref(), Expression::Identifier(_))
            }
            None => false,
        }
    }
}
