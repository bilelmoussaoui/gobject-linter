use gobject_ast::model::{FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGAsciiFunctions;

/// Maps locale-dependent C ctype/string functions to their GLib ASCII-safe
/// equivalents
fn g_ascii_replacement(func_name: &str) -> Option<&'static str> {
    match func_name {
        "tolower" => Some("g_ascii_tolower"),
        "toupper" => Some("g_ascii_toupper"),
        "isdigit" => Some("g_ascii_isdigit"),
        "isalpha" => Some("g_ascii_isalpha"),
        "isalnum" => Some("g_ascii_isalnum"),
        "isspace" => Some("g_ascii_isspace"),
        "isupper" => Some("g_ascii_isupper"),
        "islower" => Some("g_ascii_islower"),
        "isxdigit" => Some("g_ascii_isxdigit"),
        "ispunct" => Some("g_ascii_ispunct"),
        "isprint" => Some("g_ascii_isprint"),
        "isgraph" => Some("g_ascii_isgraph"),
        "iscntrl" => Some("g_ascii_iscntrl"),
        _ => None,
    }
}

impl Rule for UseGAsciiFunctions {
    fn name(&self) -> &'static str {
        "use_g_ascii_functions"
    }

    fn description(&self) -> &'static str {
        "Use g_ascii_* functions instead of locale-dependent C ctype functions"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let source = &file.source;
        for call in func.find_calls(&[
            "tolower", "toupper", "isdigit", "isalpha", "isalnum", "isspace", "isupper", "islower",
            "isxdigit", "ispunct", "isprint", "isgraph", "iscntrl",
        ]) {
            if let Some(func_name) = call.function_name_str()
                && let Some(replacement) = g_ascii_replacement(func_name)
            {
                let fix = Fix::new(
                    call.location.start_byte,
                    call.location.end_byte,
                    format!(
                        "{} ({})",
                        replacement,
                        call.arguments
                            .iter()
                            .filter_map(|arg| arg.to_source_string(source))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );

                violations.push(self.violation_with_fix(
                    &file.path,
                    call.location.line,
                    call.location.column,
                    format!(
                        "Use {}() instead of {}() — C ctype functions are locale-dependent",
                        replacement, func_name
                    ),
                    fix,
                ));
            }
        }
    }
}
