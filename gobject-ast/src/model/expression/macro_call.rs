use serde::{Deserialize, Serialize};

use super::{Argument, Expression};
use crate::model::SourceLocation;

/// Represents a macro invocation that looks like a function call
/// e.g., I_("string"), N_("string"), G_STRINGIFY(foo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroCallExpression {
    pub macro_name: String,
    pub arguments: Vec<Argument>,
    pub location: SourceLocation,
}

impl MacroCallExpression {
    /// Get argument text from source
    pub fn get_arg_text(&self, index: usize, source: &[u8]) -> Option<String> {
        self.arguments.get(index)?.to_source_string(source)
    }

    /// Extract string literal from first argument if present
    /// Common pattern: I_("string"), N_("string")
    pub fn extract_string_literal(&self) -> Option<&str> {
        if let Some(Argument::Expression(expr)) = self.arguments.first()
            && let Expression::StringLiteral(lit) = expr.as_ref()
        {
            return Some(lit.value.trim_matches('"'));
        }
        None
    }
}
