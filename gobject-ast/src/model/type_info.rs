use serde::Serialize;

fn is_zero(v: &usize) -> bool {
    *v == 0
}

use crate::model::{SourceLocation, types::BasicType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoCleanupMacro {
    /// g_autoptr(TypeName)
    Autoptr(String),
    /// g_auto(TypeName)
    Auto(String),
    /// g_autofree
    Autofree,
    /// g_autolist(TypeName)
    Autolist(String),
    /// g_autoslist(TypeName)
    Autoslist(String),
    /// g_autoqueue(TypeName)
    Autoqueue(String),
}

impl AutoCleanupMacro {
    /// Get the macro name as it would appear in documentation
    pub fn name(&self) -> &'static str {
        match self {
            Self::Autoptr(_) => "g_autoptr",
            Self::Auto(_) => "g_auto",
            Self::Autofree => "g_autofree",
            Self::Autolist(_) => "g_autolist",
            Self::Autoslist(_) => "g_autoslist",
            Self::Autoqueue(_) => "g_autoqueue",
        }
    }

    /// Get the type argument for macros that take one (None for g_autofree)
    pub fn type_arg(&self) -> Option<&str> {
        match self {
            Self::Autoptr(t)
            | Self::Auto(t)
            | Self::Autolist(t)
            | Self::Autoslist(t)
            | Self::Autoqueue(t) => Some(t),
            Self::Autofree => None,
        }
    }
}

impl std::fmt::Display for AutoCleanupMacro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Autoptr(t) => f.write_fmt(format_args!("g_autoptr({t})")),
            Self::Auto(t) => f.write_fmt(format_args!("g_auto({t})")),
            Self::Autofree => f.write_str("g_autofree"),
            Self::Autolist(t) => f.write_fmt(format_args!("g_autolist({t})")),
            Self::Autoslist(t) => f.write_fmt(format_args!("g_autoslist({t})")),
            Self::Autoqueue(t) => f.write_fmt(format_args!("g_autoqueue({t})")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    /// Base type without qualifiers or pointers: `"GFile"`, `"int"`.
    pub base_type: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_const: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_volatile: bool,
    /// True when spelled with the `struct` keyword (`struct Foo *`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_struct: bool,
    /// True when spelled with the `union` keyword (`union Foo *`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_union: bool,
    /// Pointer indirections: 0 = value, 1 = `*`, 2 = `**`.
    #[serde(skip_serializing_if = "is_zero")]
    pub pointer_depth: usize,
    pub location: SourceLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_cleanup: Option<AutoCleanupMacro>,
}

impl TypeInfo {
    pub fn new(type_string: &str, location: SourceLocation) -> Self {
        let trimmed = type_string.trim();
        let auto_cleanup = Self::parse_auto_cleanup(trimmed);

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let mut filtered_parts: Vec<&str> = Vec::new();
        let mut is_const = false;

        let macro_name = auto_cleanup.as_ref().map(AutoCleanupMacro::name);
        let mut is_volatile = false;

        for part in &parts {
            match *part {
                "static" | "extern" | "inline" => {}
                "const" => {
                    is_const = true;
                    filtered_parts.push(part);
                }
                "volatile" => {
                    is_volatile = true;
                    filtered_parts.push(part);
                }
                // Drop the macro token(s): bare name ("g_autoptr"), compact
                // form ("g_autoptr(Foo)"), or the arg token ("(Foo)") from
                // the spaced form "g_autoptr (Foo)".
                _ if macro_name
                    .is_some_and(|name| part.starts_with(name) || part.starts_with('(')) => {}
                _ => {
                    filtered_parts.push(part);
                }
            }
        }

        let cleaned = filtered_parts.join(" ");

        // Strip const/volatile in any order to reach the base type.
        let without_qualifiers: String = cleaned
            .split_whitespace()
            .filter(|&w| w != "const" && w != "volatile")
            .collect::<Vec<_>>()
            .join(" ");

        let pointer_depth = without_qualifiers.chars().filter(|&c| c == '*').count();

        let raw_base = without_qualifiers.replace('*', "").trim().to_string();
        let (base_type, is_struct, is_union) = if let Some(ref auto) = auto_cleanup {
            if let Some(type_arg) = auto.type_arg() {
                (type_arg.to_owned(), false, false)
            } else {
                Self::extract_base_type(&raw_base)
            }
        } else {
            Self::extract_base_type(&raw_base)
        };

        Self {
            base_type,
            is_const,
            is_volatile,
            is_struct,
            is_union,
            pointer_depth,
            location,
            auto_cleanup,
        }
    }

    fn extract_base_type(raw_base: &str) -> (String, bool, bool) {
        if let Some(rest) = raw_base.strip_prefix("struct ") {
            (rest.trim().to_string(), true, false)
        } else if let Some(rest) = raw_base.strip_prefix("union ") {
            (rest.trim().to_string(), false, true)
        } else {
            (raw_base.to_string(), false, false)
        }
    }

    fn parse_auto_cleanup(type_str: &str) -> Option<AutoCleanupMacro> {
        let try_with_arg = |macro_name: &str| -> Option<String> {
            let pos = type_str.find(macro_name)?;
            let after_name = type_str[pos + macro_name.len()..].trim_start();
            if !after_name.starts_with('(') {
                return None;
            }
            let inner = &after_name[1..];
            let end = inner.find(')')?;
            Some(inner[..end].trim().to_string())
        };

        if type_str.contains("g_autofree") {
            Some(AutoCleanupMacro::Autofree)
        } else if let Some(t) = try_with_arg("g_autoptr") {
            Some(AutoCleanupMacro::Autoptr(t))
        } else if let Some(t) = try_with_arg("g_autolist") {
            Some(AutoCleanupMacro::Autolist(t))
        } else if let Some(t) = try_with_arg("g_autoslist") {
            Some(AutoCleanupMacro::Autoslist(t))
        } else if let Some(t) = try_with_arg("g_autoqueue") {
            Some(AutoCleanupMacro::Autoqueue(t))
        } else {
            try_with_arg("g_auto").map(AutoCleanupMacro::Auto)
        }
    }

    /// Check if this is a pointer type (at least one level of indirection)
    pub fn is_pointer(&self) -> bool {
        self.pointer_depth > 0
    }

    /// Get the base type without any qualifiers or pointers
    pub fn base_type_name(&self) -> &str {
        &self.base_type
    }

    /// Check if the base type matches the given name
    pub fn is_base_type(&self, name: &str) -> bool {
        self.base_type == name
    }

    /// Human-readable type string reconstructed from structured fields,
    /// e.g. `"const char *"`.
    pub fn display_name(&self) -> String {
        let mut s = String::new();
        if self.is_const {
            s.push_str("const ");
        }
        if self.is_struct {
            s.push_str("struct ");
        } else if self.is_union {
            s.push_str("union ");
        }
        s.push_str(&self.base_type);
        if self.pointer_depth > 0 {
            s.push(' ');
            for _ in 0..self.pointer_depth {
                s.push('*');
            }
        }
        s
    }

    /// Check if the type uses any auto-cleanup macro (g_autoptr, g_autofree,
    /// g_autolist, etc.)
    pub fn uses_auto_cleanup(&self) -> bool {
        self.auto_cleanup.is_some()
    }

    /// GLib C aliases normalised to their C equivalents (`gint` → `int`).
    pub fn normalized_base_type(&self) -> &str {
        match self.base_type.as_str() {
            "gint" => "int",
            "guint" => "unsigned int",
            "glong" => "long",
            "gulong" => "unsigned long",
            "gshort" => "short",
            "gushort" => "unsigned short",
            "gchar" => "char",
            "guchar" => "unsigned char",
            "gfloat" => "float",
            "gdouble" => "double",
            other => other,
        }
    }

    /// Return true if `self` and `other` represent the same type, treating
    /// GLib C aliases as equivalent to their underlying C types.
    pub fn matches(&self, other: &Self) -> bool {
        self.normalized_base_type() == other.normalized_base_type()
            && self.pointer_depth == other.pointer_depth
            && self.is_const == other.is_const
    }

    /// Returns the `BasicType` if this C type is a primitive scalar.
    /// Handles GLib names (`gint`), C equivalents (`int`), C99 fixed-width
    /// types (`int32_t`), and C99 types (`long long`, `_Bool`).
    /// `gchar *` / `char *` at pointer_depth 1 maps to `BasicType::String`.
    pub fn as_basic(&self) -> Option<BasicType> {
        // Pointer-depth-sensitive entries first: these override the base-name
        // match below (e.g. `char *` is String, not Char).
        match (self.base_type.as_str(), self.pointer_depth) {
            ("gchar" | "char", 1) => return Some(BasicType::String),
            ("gpointer" | "void", 1) => return Some(BasicType::Pointer),
            ("gconstpointer", 0) => return Some(BasicType::Pointer),
            _ => {}
        }

        // All remaining matches depend only on the base type name.
        match self.base_type.as_str() {
            "gboolean" => Some(BasicType::Boolean),
            "gchar" | "char" => Some(BasicType::Char),
            "guchar" | "unsigned char" => Some(BasicType::UChar),
            "gint" | "int" | "signed" | "signed int" => Some(BasicType::Int),
            "guint" | "unsigned int" | "unsigned" => Some(BasicType::UInt),
            "glong" | "long" | "signed long" | "long int" | "signed long int" => {
                Some(BasicType::Long)
            }
            "gulong" | "unsigned long" | "unsigned long int" => Some(BasicType::ULong),
            "gint64" | "int64_t" => Some(BasicType::Int64),
            "guint64" | "uint64_t" => Some(BasicType::UInt64),
            "gfloat" | "float" => Some(BasicType::Float),
            "gdouble" | "double" => Some(BasicType::Double),
            "_Bool" | "bool" => Some(BasicType::Bool),
            "gshort" | "short" | "signed short" | "short int" | "signed short int" => {
                Some(BasicType::Short)
            }
            "gushort" | "unsigned short" | "unsigned short int" => Some(BasicType::UShort),
            "long long" | "signed long long" | "long long int" | "signed long long int" => {
                Some(BasicType::LongLong)
            }
            "unsigned long long" | "unsigned long long int" => Some(BasicType::ULongLong),
            "long double" => Some(BasicType::LongDouble),
            "gint8" | "int8_t" | "signed char" => Some(BasicType::Int8),
            "guint8" | "uint8_t" => Some(BasicType::UInt8),
            "gint16" | "int16_t" => Some(BasicType::Int16),
            "guint16" | "uint16_t" => Some(BasicType::UInt16),
            "gint32" | "int32_t" => Some(BasicType::Int32),
            "guint32" | "uint32_t" => Some(BasicType::UInt32),
            "gsize" | "size_t" => Some(BasicType::Size),
            "gssize" | "ssize_t" => Some(BasicType::SSize),
            "goffset" => Some(BasicType::Offset),
            "gintptr" | "intptr_t" => Some(BasicType::IntPtr),
            "guintptr" | "uintptr_t" => Some(BasicType::UIntPtr),
            _ => None,
        }
    }

    pub fn is_basic(&self) -> bool {
        matches!(
            self.base_type.as_str(),
            "gboolean"
                | "gchar"
                | "char"
                | "guchar"
                | "unsigned char"
                | "gint"
                | "int"
                | "signed"
                | "signed int"
                | "guint"
                | "unsigned int"
                | "unsigned"
                | "glong"
                | "long"
                | "signed long"
                | "long int"
                | "signed long int"
                | "gulong"
                | "unsigned long"
                | "unsigned long int"
                | "gint64"
                | "int64_t"
                | "guint64"
                | "uint64_t"
                | "gfloat"
                | "float"
                | "gdouble"
                | "double"
                | "gpointer"
                | "void"
                | "gconstpointer"
                | "_Bool"
                | "bool"
                | "gshort"
                | "short"
                | "signed short"
                | "short int"
                | "signed short int"
                | "gushort"
                | "unsigned short"
                | "unsigned short int"
                | "long long"
                | "signed long long"
                | "long long int"
                | "signed long long int"
                | "unsigned long long"
                | "unsigned long long int"
                | "long double"
                | "gint8"
                | "int8_t"
                | "signed char"
                | "guint8"
                | "uint8_t"
                | "gint16"
                | "int16_t"
                | "guint16"
                | "uint16_t"
                | "gint32"
                | "int32_t"
                | "guint32"
                | "uint32_t"
                | "gsize"
                | "size_t"
                | "gssize"
                | "ssize_t"
                | "goffset"
                | "gintptr"
                | "intptr_t"
                | "guintptr"
                | "uintptr_t"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type_info() {
        let t = TypeInfo::new("g_autofree char *", SourceLocation::default());
        assert_eq!(t.auto_cleanup, Some(AutoCleanupMacro::Autofree));
        assert_eq!(t.base_type, "char");
        assert_eq!(t.pointer_depth, 1);

        let t = TypeInfo::new(
            "g_autofree FuZipFirmwareWriteItem *",
            SourceLocation::default(),
        );
        assert_eq!(t.auto_cleanup, Some(AutoCleanupMacro::Autofree));
        assert_eq!(t.base_type, "FuZipFirmwareWriteItem");

        let t = TypeInfo::new("const g_autofree char *", SourceLocation::default());
        assert_eq!(t.auto_cleanup, Some(AutoCleanupMacro::Autofree));
        assert!(t.is_const);
        assert_eq!(t.base_type, "char");

        let t = TypeInfo::new("g_autoptr(GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoptr("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("g_autoptr (GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoptr("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new(
            "g_autoptr (GdmConfigCommandHandler)",
            SourceLocation::default(),
        );
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoptr("GdmConfigCommandHandler".into()))
        );
        assert_eq!(t.base_type, "GdmConfigCommandHandler");

        let t = TypeInfo::new("g_auto(GString)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Auto("GString".into()))
        );
        assert_eq!(t.base_type, "GString");

        let t = TypeInfo::new("g_auto (GString)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Auto("GString".into()))
        );
        assert_eq!(t.base_type, "GString");

        let t = TypeInfo::new("g_autolist(GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autolist("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("g_autolist (GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autolist("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("g_autoslist(GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoslist("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("g_autoslist (GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoslist("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("g_autoqueue(GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoqueue("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("g_autoqueue (GFile)", SourceLocation::default());
        assert_eq!(
            t.auto_cleanup,
            Some(AutoCleanupMacro::Autoqueue("GFile".into()))
        );
        assert_eq!(t.base_type, "GFile");

        let t = TypeInfo::new("char *", SourceLocation::default());
        assert_eq!(t.auto_cleanup, None);
        assert_eq!(t.base_type, "char");
        assert_eq!(t.pointer_depth, 1);
    }
}
