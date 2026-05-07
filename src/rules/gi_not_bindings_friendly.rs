use gobject_ast::{
    model::{doc::FunctionAnnotation, top_level::FunctionDeclItem},
    types::Parameter,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

const GLIB_CONTAINER_TYPES: &[&str] = &[
    "GList",
    "GSList",
    "GHashTable",
    "GPtrArray",
    "GArray",
    "GByteArray",
];

pub struct GiNotBindingsFriendly;

impl Rule for GiNotBindingsFriendly {
    fn name(&self) -> &'static str {
        "gi_not_bindings_friendly"
    }

    fn description(&self) -> &'static str {
        "Detect public API patterns that are problematic for GObject Introspection bindings"
    }

    fn category(&self) -> Category {
        Category::Introspection
    }

    fn requires_meson(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        if !ast_context.has_public_private_info() {
            return;
        }

        for (path, file) in ast_context.iter_header_files() {
            if !ast_context.is_gir_header(path).unwrap_or(false) {
                continue;
            }

            for func in file.iter_function_declarations() {
                if func.export_macros.is_empty() {
                    continue;
                }
                if is_skipped(func) {
                    continue;
                }
                if func.name.ends_with("_get_type") || func.name.ends_with("_error_quark") {
                    continue;
                }

                self.check_variadic(func, path, violations);
                self.check_out_params(func, path, violations);
                self.check_container_types(func, path, violations);
            }
        }
    }
}

fn is_skipped(func: &FunctionDeclItem) -> bool {
    func.doc
        .as_ref()
        .is_some_and(|d| d.annotations.contains(&FunctionAnnotation::Skip))
}

impl GiNotBindingsFriendly {
    fn check_variadic(
        &self,
        func: &FunctionDeclItem,
        path: &std::path::Path,
        violations: &mut Vec<Violation>,
    ) {
        if func
            .parameters
            .iter()
            .any(|p| matches!(p, Parameter::Variadic))
        {
            violations.push(self.violation(
                path,
                func.location.line,
                func.location.column,
                format!(
                    "Function '{}' uses variadic arguments which cannot be introspected",
                    func.name,
                ),
            ));
        }
    }

    fn check_out_params(
        &self,
        func: &FunctionDeclItem,
        path: &std::path::Path,
        violations: &mut Vec<Violation>,
    ) {
        let params = &func.parameters;
        if params.is_empty() {
            return;
        }

        let mut out_count = 0usize;

        for (i, param) in params.iter().enumerate() {
            let Parameter::Regular { type_info, .. } = param else {
                continue;
            };
            // Skip first param (self)
            if i == 0 {
                continue;
            }
            // Skip GError ** as last param
            if i == params.len() - 1
                && type_info.base_type == "GError"
                && type_info.pointer_depth >= 2
            {
                continue;
            }
            let min_depth = if type_info.is_basic() { 1 } else { 2 };
            if type_info.pointer_depth >= min_depth {
                out_count += 1;
            }
        }

        if out_count > 2 {
            violations.push(self.violation(
                path,
                func.location.line,
                func.location.column,
                format!(
                    "Function '{}' has {} out parameters (pointer-to-pointer); \
                     consider reducing to at most 2 for better bindings",
                    func.name, out_count,
                ),
            ));
        }
    }

    fn check_container_types(
        &self,
        func: &FunctionDeclItem,
        path: &std::path::Path,
        violations: &mut Vec<Violation>,
    ) {
        if is_container_type(&func.return_type.base_type) && func.return_type.pointer_depth >= 1 {
            violations.push(self.violation(
                path,
                func.location.line,
                func.location.column,
                format!(
                    "Function '{}' returns {}* in public API; {}",
                    func.name,
                    func.return_type.base_type,
                    container_suggestion(&func.return_type.base_type),
                ),
            ));
        }

        for param in &func.parameters {
            let Parameter::Regular {
                type_info, name, ..
            } = param
            else {
                continue;
            };
            if is_container_type(&type_info.base_type) && type_info.pointer_depth >= 1 {
                let param_label = name.as_deref().unwrap_or("(unnamed)");
                violations.push(self.violation(
                    path,
                    func.location.line,
                    func.location.column,
                    format!(
                        "Function '{}' parameter '{}' uses {}* in public API; {}",
                        func.name,
                        param_label,
                        type_info.base_type,
                        container_suggestion(&type_info.base_type),
                    ),
                ));
            }
        }
    }
}

fn is_container_type(base_type: &str) -> bool {
    GLIB_CONTAINER_TYPES.contains(&base_type)
}

fn container_suggestion(base_type: &str) -> &'static str {
    match base_type {
        "GList" | "GSList" => "Consider GListModel for better introspection support",
        "GHashTable" => "Introspection cannot determine key/value types",
        _ => "Consider a typed alternative for better introspection support",
    }
}
