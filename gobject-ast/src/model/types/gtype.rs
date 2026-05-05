use serde::{Deserialize, Serialize};

use crate::model::expression::Expression;

/// A GLib GType reference — either a macro/define or a `_get_type()` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GType {
    /// `G_TYPE_NONE` — void / no return value.
    None,
    /// A macro or `#define` like `G_TYPE_BOOLEAN` or `GTK_TYPE_WIDGET`.
    Identifier(String),
    /// A `_get_type()` function call; stores the function name (e.g.
    /// `"gtk_widget_get_type"`).
    Call(String),
}

impl GType {
    pub fn is_none(&self) -> bool {
        matches!(self, GType::None)
    }

    pub fn from_expression(expr: &Expression) -> Option<Self> {
        match expr {
            Expression::Identifier(id) if id.name == "G_TYPE_NONE" => Some(GType::None),
            Expression::Identifier(id) => Some(GType::Identifier(id.name.clone())),
            Expression::Call(call) => Some(GType::Call(call.function_name())),
            _ => None,
        }
    }
}
