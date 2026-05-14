use std::collections::HashSet;

use gobject_ast::model::{Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

fn gsource_callback_arg_index(func_name: &str) -> Option<usize> {
    match func_name {
        "g_idle_add" => Some(0),
        "g_idle_add_full"
        | "g_timeout_add"
        | "g_timeout_add_seconds"
        | "gtk_widget_add_tick_callback" => Some(1),
        "g_timeout_add_full" | "g_timeout_add_seconds_full" => Some(2),
        _ => None,
    }
}

pub struct UseGSourceConstants;

impl Rule for UseGSourceConstants {
    fn name(&self) -> &'static str {
        "use_g_source_constants"
    }

    fn description(&self) -> &'static str {
        "Use G_SOURCE_CONTINUE/G_SOURCE_REMOVE instead of TRUE/FALSE in GSourceFunc callbacks"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let mut callbacks: HashSet<&str> = HashSet::new();

        for (_path, file) in ast_context.iter_c_files() {
            for func in file.iter_function_definitions() {
                for call in
                    func.find_calls_matching(|name| gsource_callback_arg_index(name).is_some())
                {
                    if let Some(idx) = gsource_callback_arg_index(call.function_name_str().unwrap())
                        && let Some(name) =
                            call.get_arg(idx).and_then(|a| a.extract_identifier_name())
                    {
                        callbacks.insert(name);
                    }
                }
            }
        }

        if callbacks.is_empty() {
            return;
        }

        for (path, file) in ast_context.iter_c_files() {
            for func in file.iter_function_definitions() {
                if callbacks.contains(func.name.as_str()) {
                    self.check_statements(path, &func.body_statements, violations);
                }
            }
        }
    }
}

impl UseGSourceConstants {
    fn check_statements(
        &self,
        file_path: &std::path::Path,
        statements: &[Statement],
        violations: &mut Vec<Violation>,
    ) {
        for stmt in statements {
            for ret_stmt in stmt.iter_returns() {
                if let Some(value) = &ret_stmt.value {
                    self.check_return_value(file_path, value, violations);
                }
            }
        }
    }

    fn check_return_value(
        &self,
        file_path: &std::path::Path,
        expr: &Expression,
        violations: &mut Vec<Violation>,
    ) {
        expr.walk(&mut |e| {
            let (old_name, replacement) = if e.is_truthy() {
                ("TRUE", "G_SOURCE_CONTINUE")
            } else if e.is_falsy() {
                ("FALSE", "G_SOURCE_REMOVE")
            } else {
                return;
            };

            let loc = e.location();
            let message = format!(
                "Use {} instead of {} in GSourceFunc callback",
                replacement, old_name
            );
            let fix = Fix::new(loc.start_byte, loc.end_byte, replacement);

            violations.push(self.violation_with_fix_at(file_path, loc, message, fix));
        });
    }
}
