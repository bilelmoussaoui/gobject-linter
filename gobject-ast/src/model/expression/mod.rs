mod alloc_call;
mod assignment;
mod binary;
mod call;
mod cast;
mod conditional;
mod field_access;
mod identifier;
mod initializer_list;
mod literal;
mod offsetof;
mod sizeof;
mod subscript;
mod unary;
mod update;

use std::collections::HashMap;

pub use alloc_call::AllocCallExpression;
pub use assignment::Assignment;
pub use binary::BinaryExpression;
pub use call::CallExpression;
pub use cast::CastExpression;
pub use conditional::ConditionalExpression;
pub use field_access::FieldAccessExpression;
pub use identifier::IdentifierExpression;
pub use initializer_list::{Designator, InitializerItem, InitializerListExpression};
pub use literal::{
    BooleanExpression, CharLiteralExpression, CommentExpression, GenericExpression, NullExpression,
    NumberLiteralExpression, StringLiteralExpression,
};
pub use offsetof::{OffsetField, OffsetOfExpression};
use serde::Serialize;
pub use sizeof::{SizeofExpression, SizeofOperand};
pub use subscript::SubscriptExpression;
pub use unary::UnaryExpression;
pub use update::UpdateExpression;

use crate::model::{DefineValue, SourceLocation};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Expression {
    Call(CallExpression),
    AllocCall(AllocCallExpression),
    Assignment(Assignment),
    Binary(BinaryExpression),
    Unary(UnaryExpression),
    Identifier(IdentifierExpression),
    FieldAccess(FieldAccessExpression),
    StringLiteral(StringLiteralExpression),
    NumberLiteral(NumberLiteralExpression),
    Null(NullExpression),
    Boolean(BooleanExpression),
    Cast(CastExpression),
    Conditional(ConditionalExpression),
    Sizeof(SizeofExpression),
    Subscript(SubscriptExpression),
    InitializerList(InitializerListExpression),
    CharLiteral(CharLiteralExpression),
    Update(UpdateExpression),
    Comment(CommentExpression),
    OffsetOf(OffsetOfExpression),
    Generic(GenericExpression),
}

impl Expression {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Self::Call(c) => &c.location,
            Self::AllocCall(a) => &a.location,
            Self::Assignment(a) => &a.location,
            Self::Binary(b) => &b.location,
            Self::Unary(u) => &u.location,
            Self::Identifier(i) => &i.location,
            Self::FieldAccess(f) => &f.location,
            Self::StringLiteral(s) => &s.location,
            Self::NumberLiteral(n) => &n.location,
            Self::Null(n) => &n.location,
            Self::Boolean(b) => &b.location,
            Self::Cast(c) => &c.location,
            Self::Conditional(c) => &c.location,
            Self::Sizeof(s) => &s.location,
            Self::Subscript(s) => &s.location,
            Self::InitializerList(i) => &i.location,
            Self::CharLiteral(c) => &c.location,
            Self::Update(u) => &u.location,
            Self::Comment(c) => &c.location,
            Self::OffsetOf(o) => &o.location,
            Self::Generic(g) => &g.location,
        }
    }

    pub fn is_falsy(&self) -> bool {
        match self {
            Self::Boolean(b) => !b.value,
            Self::NumberLiteral(n) => n.value == "0",
            Self::Identifier(id) => id.name == "FALSE",
            _ => false,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Boolean(b) => b.value,
            Self::NumberLiteral(n) => n.value == "1",
            Self::Identifier(id) => id.name == "TRUE",
            _ => false,
        }
    }

    /// Recursively walk all nested expressions. The closure receives a
    /// `&'s Expression` tied to `self`'s lifetime, so references extracted
    /// inside the closure can be stored in an outer `Vec<&'s T>`.
    pub fn walk<'s, F>(&'s self, f: &mut F)
    where
        F: FnMut(&'s Self),
    {
        f(self);
        match self {
            Self::Call(call) => {
                call.function.walk(f);
                for arg in &call.arguments {
                    arg.walk(f);
                }
            }
            Self::AllocCall(alloc) => {
                alloc.function.walk(f);
                for arg in &alloc.arguments {
                    arg.walk(f);
                }
            }
            Self::Assignment(assign) => {
                assign.lhs.walk(f);
                assign.rhs.walk(f);
            }
            Self::Unary(unary) => {
                unary.operand.walk(f);
            }
            Self::Binary(binary) => {
                binary.left.walk(f);
                binary.right.walk(f);
            }
            Self::Cast(cast) => {
                cast.operand.walk(f);
            }
            Self::Conditional(cond) => {
                cond.condition.walk(f);
                cond.then_expr.walk(f);
                cond.else_expr.walk(f);
            }
            Self::Subscript(subscript) => {
                subscript.array.walk(f);
                subscript.index.walk(f);
            }
            Self::Update(update) => {
                update.operand.walk(f);
            }
            Self::FieldAccess(field) => {
                field.base.walk(f);
            }
            Self::InitializerList(init) => {
                for item in &init.items {
                    if let Some(Designator::Subscript(idx)) = &item.designator {
                        idx.walk(f);
                    }
                    item.value.walk(f);
                }
            }
            Self::Identifier(_)
            | Self::StringLiteral(_)
            | Self::NumberLiteral(_)
            | Self::Null(_)
            | Self::Boolean(_)
            | Self::Sizeof(_)
            | Self::CharLiteral(_)
            | Self::Comment(_)
            | Self::OffsetOf(_)
            | Self::Generic(_) => {}
        }
    }

    /// Extract variable name from simple expressions (Identifier or
    /// FieldAccess)
    pub fn extract_variable_name(&self) -> Option<&str> {
        match self {
            Self::Identifier(_) | Self::FieldAccess(_) => self.location().as_str(),
            _ => None,
        }
    }

    /// Extract the identifier name, unwrapping macro calls and casts.
    /// `G_OBJECT(self)` → `"self"`, `(GSourceFunc) callback` → `"callback"`
    pub fn extract_identifier_name(&self) -> Option<&str> {
        match self {
            Self::Identifier(id) => Some(&id.name),
            Self::Call(call) => call.get_arg(0)?.extract_identifier_name(),
            Self::Cast(cast) => cast.operand.extract_identifier_name(),
            _ => None,
        }
    }

    /// Check if this expression is NULL
    /// Handles both Expression::Null and the identifier "NULL" (common in C
    /// code)
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null(_)) || matches!(self, Self::Identifier(id) if id.name == "NULL")
    }

    /// Check if this expression is the number 0
    pub fn is_zero(&self) -> bool {
        matches!(self, Self::NumberLiteral(n) if n.value.trim() == "0")
    }

    /// Check if this expression is a string literal
    pub fn is_string_literal(&self) -> bool {
        matches!(self, Self::StringLiteral(_))
    }

    /// Extract string literal value, unwrapping macro calls like I_("string")
    /// Returns the string without quotes
    pub fn extract_string_value(&self) -> Option<String> {
        match self {
            Self::StringLiteral(lit) => Some(lit.value.trim_matches('"').to_string()),
            Self::Call(call) => call.get_arg(0)?.extract_string_value(),
            _ => None,
        }
    }

    /// Like `extract_string_value`, but also resolves identifiers through a
    /// define map (e.g. `MY_PROP_NAME` → the string from `#define MY_PROP_NAME
    /// "foo"`).
    pub fn resolve_string_value(&self, defines: &HashMap<String, DefineValue>) -> Option<String> {
        if let Some(s) = self.extract_string_value() {
            return Some(s);
        }
        match self {
            Self::Identifier(id) => match defines.get(&id.name)? {
                DefineValue::StringLiteral(s) => Some(s.clone()),
                _ => None,
            },
            Self::Call(call) => {
                let arg = call.get_arg(0)?;
                if let Self::Identifier(id) = arg {
                    match defines.get(&id.name)? {
                        DefineValue::StringLiteral(s) => Some(s.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if this is a string literal or a macro wrapping a string literal
    pub fn is_string_or_macro_string(&self) -> bool {
        self.extract_string_value().is_some()
    }

    /// Check if this expression contains an identifier with the given name
    /// Recursively searches through the entire expression tree
    pub fn contains_identifier(&self, name: &str) -> bool {
        let mut found = false;
        self.walk(&mut |e| {
            if let Self::Identifier(id) = e
                && id.name == name
            {
                found = true;
            }
        });
        found
    }

    /// Collect all identifiers in this expression
    /// Returns a list of all identifier names found in the expression tree
    pub fn collect_identifiers(&self) -> Vec<String> {
        let mut identifiers = Vec::new();
        self.walk(&mut |e| {
            if let Self::Identifier(id) = e {
                identifiers.push(id.name.clone());
            }
        });
        identifiers
    }

    /// Check if this expression is a call to the specified function
    pub fn is_call_to(&self, function_name: &str) -> bool {
        matches!(self, Self::Call(call) if call.is_function(function_name))
    }

    /// Check if this expression is a call to any of the specified functions
    pub fn is_call_to_any(&self, function_names: &[&str]) -> bool {
        matches!(self, Self::Call(call) if call.function_name_str().is_some_and(|name| function_names.contains(&name)))
    }
}
