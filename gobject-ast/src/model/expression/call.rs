use serde::Serialize;

use crate::model::{Expression, SourceLocation};

#[derive(Debug, Clone, Serialize)]
pub struct CallExpression {
    pub function: Box<Expression>, // Can be Identifier or FieldAccess
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<Argument>,
    pub location: SourceLocation,
}

impl CallExpression {
    /// Get the function name as a string
    /// For identifiers, returns the name
    /// For field access, returns the full text (e.g., "parent_class->dispose")
    pub fn function_name<'a>(&self, source: &'a [u8]) -> &'a str {
        self.function.location().as_str(source).unwrap_or("")
    }

    /// Check if the function matches a specific name
    pub fn is_function(&self, name: &str) -> bool {
        match &*self.function {
            Expression::Identifier(id) => id.name == name,
            _ => false,
        }
    }

    /// Check if function name contains a pattern
    pub fn function_contains(&self, pattern: &str, source: &[u8]) -> bool {
        self.function_name(source).contains(pattern)
    }

    /// Check if function name ends with a pattern
    pub fn function_ends_with(&self, pattern: &str, source: &[u8]) -> bool {
        self.function_name(source).ends_with(pattern)
    }

    /// Get the function name as &str (for common case of identifier)
    /// Returns None if function is not a simple identifier
    pub fn function_name_str(&self) -> Option<&str> {
        match &*self.function {
            Expression::Identifier(id) => Some(&id.name),
            _ => None,
        }
    }

    /// Get the expression for the argument at the given index
    /// Automatically unwraps Argument::Expression
    pub fn get_arg(&self, index: usize) -> Option<&Expression> {
        match self.arguments.get(index)? {
            Argument::Expression(expr) => Some(expr.as_ref()),
        }
    }

    /// Get argument as source text
    pub fn get_arg_text<'a>(&self, index: usize, source: &'a [u8]) -> Option<&'a str> {
        match self.arguments.get(index)? {
            Argument::Expression(expr) => expr.to_source_string(source),
        }
    }

    /// Check if the argument at the given index exists and matches the
    /// predicate
    pub fn has_arg_matching<F>(&self, index: usize, predicate: F) -> bool
    where
        F: FnOnce(&Expression) -> bool,
    {
        self.get_arg(index).is_some_and(predicate)
    }

    /// Check if the argument at the given index contains a reference to the
    /// specified variable Handles both plain identifiers and field access
    /// (e.g., obj->field)
    pub fn arg_contains_variable(&self, index: usize, var_name: &str, source: &[u8]) -> bool {
        self.has_arg_matching(index, |expr| {
            expr.extract_variable_name(source)
                .is_some_and(|name| name == var_name)
        })
    }

    /// Check if this looks like a macro call (ALL_CAPS or ends with _)
    /// Examples: I_, N_, G_STRINGIFY, GINT_TO_POINTER
    pub fn is_likely_macro(&self, source: &[u8]) -> bool {
        let name = self.function_name(source);
        name.chars().all(|c| c.is_uppercase() || c == '_') || name.ends_with('_')
    }

    /// Extract string literal from argument, unwrapping macro calls like
    /// I_("string") This is useful for g_param_spec calls where the name
    /// might be I_("property-name")
    pub fn extract_string_from_arg(&self, index: usize) -> Option<String> {
        let Argument::Expression(expr) = self.arguments.get(index)?;
        expr.extract_string_value()
    }

    /// Check if this call is a GObject allocation function
    /// Recognizes g_object_new, g_new, and various other allocation patterns
    pub fn is_allocation_call(&self) -> bool {
        if let Some(name) = self.function_name_str() {
            matches!(
                name,
                "g_object_new"
                    | "g_object_new_with_properties"
                    | "g_type_create_instance"
                    | "g_new"
                    | "g_new0"
                    | "g_try_new"
                    | "g_try_new0"
                    | "g_malloc"
                    | "g_malloc0"
                    | "g_strdup"
                    | "g_strndup"
                    | "g_file_new_for_path"
                    | "g_file_new_for_uri"
                    | "g_file_new_tmp"
                    | "g_variant_new"
                    | "g_variant_ref_sink"
                    | "g_bytes_new"
                    | "g_bytes_new_take"
                    | "g_hash_table_new"
                    | "g_hash_table_new_full"
                    | "g_array_new"
                    | "g_ptr_array_new"
                    | "g_error_new"
                    | "g_error_new_literal"
            ) || name.ends_with("_new")
                || name.ends_with("_get_instance")
                || name.contains("_new_")
                || name.contains("_create")
        } else {
            false
        }
    }

    /// Check if this call is a GObject cleanup/free function
    /// Recognizes g_object_unref, g_free, and various other cleanup patterns
    pub fn is_cleanup_call(&self) -> bool {
        if let Some(name) = self.function_name_str() {
            matches!(
                name,
                "g_object_unref"
                    | "g_clear_object"
                    | "g_clear_pointer"
                    | "g_error_free"
                    | "g_clear_error"
                    | "g_free"
                    | "g_clear_handle_id"
                    | "g_clear_signal_handler"
                    | "g_list_free"
                    | "g_list_free_full"
                    | "g_slist_free"
                    | "g_slist_free_full"
                    | "g_hash_table_unref"
                    | "g_hash_table_destroy"
                    | "g_bytes_unref"
                    | "g_variant_unref"
                    | "g_array_unref"
                    | "g_array_free"
                    | "g_ptr_array_unref"
                    | "g_ptr_array_free"
            ) || name.ends_with("_unref")
                || name.ends_with("_free")
                || name.ends_with("_destroy")
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Argument {
    Expression(Box<Expression>),
}

impl Argument {
    /// Convert this argument back to source text
    pub fn to_source_string<'a>(&self, source: &'a [u8]) -> Option<&'a str> {
        match self {
            Self::Expression(expr) => expr.to_source_string(source),
        }
    }

    /// Check if this argument is a string literal or macro wrapping a string
    pub fn is_string_or_macro_string(&self) -> bool {
        let Self::Expression(expr) = self;
        expr.is_string_or_macro_string()
    }

    /// Check if this argument is NULL
    pub fn is_null(&self) -> bool {
        let Self::Expression(expr) = self;
        expr.is_null()
    }

    /// Extract string value from this argument, unwrapping macros
    pub fn extract_string_value(&self) -> Option<String> {
        let Self::Expression(expr) = self;
        expr.extract_string_value()
    }
}
