use serde::Serialize;

use crate::model::{expression::Expression, types::BasicType};

/// A GLib GType reference — either a macro/define or a `_get_type()` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
        matches!(self, Self::None)
    }

    pub fn from_expression(expr: &Expression) -> Option<Self> {
        match expr {
            Expression::Identifier(id) if id.name == "G_TYPE_NONE" => Some(Self::None),
            Expression::Identifier(id) => Some(Self::Identifier(id.name.clone())),
            Expression::Call(call) => Some(Self::Call(call.function_name())),
            _ => None,
        }
    }

    /// Returns the `BasicType` if this GType maps to a GLib primitive, `None`
    /// for object/boxed/interface types and `_get_type()` calls.
    pub fn as_basic(&self) -> Option<BasicType> {
        let Self::Identifier(id) = self else {
            return None;
        };
        match id.as_str() {
            "G_TYPE_BOOLEAN" => Some(BasicType::Boolean),
            "G_TYPE_CHAR" => Some(BasicType::Char),
            "G_TYPE_UCHAR" => Some(BasicType::UChar),
            "G_TYPE_INT" => Some(BasicType::Int),
            "G_TYPE_UINT" => Some(BasicType::UInt),
            "G_TYPE_LONG" => Some(BasicType::Long),
            "G_TYPE_ULONG" => Some(BasicType::ULong),
            "G_TYPE_INT64" => Some(BasicType::Int64),
            "G_TYPE_UINT64" => Some(BasicType::UInt64),
            "G_TYPE_FLOAT" => Some(BasicType::Float),
            "G_TYPE_DOUBLE" => Some(BasicType::Double),
            "G_TYPE_STRING" => Some(BasicType::String),
            "G_TYPE_POINTER" => Some(BasicType::Pointer),
            _ => None,
        }
    }
}
