use std::collections::HashSet;

use gobject_ast::model::{CallExpression, Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

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
                for call in func.find_calls(&[
                    "g_idle_add",
                    "g_idle_add_full",
                    "g_timeout_add",
                    "g_timeout_add_seconds",
                    "g_timeout_add_full",
                    "g_timeout_add_seconds_full",
                    "gtk_widget_add_tick_callback",
                ]) {
                    if let Some(name) = self.extract_callback_name(call, &file.source) {
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
                    self.check_statements(path, &func.body_statements, &file.source, violations);
                }
            }
        }
    }
}

impl UseGSourceConstants {
    fn extract_callback_name<'a>(
        &self,
        call: &'a CallExpression,
        _source: &[u8],
    ) -> Option<&'a str> {
        // Map of source-add function name → zero-based index of the GSourceFunc
        // argument
        let func_name = call.function_name_str()?;
        let callback_arg_index: usize = match func_name {
            "g_idle_add" => 0,
            "g_idle_add_full"
            | "g_timeout_add"
            | "g_timeout_add_seconds"
            | "gtk_widget_add_tick_callback" => 1,
            "g_timeout_add_full" | "g_timeout_add_seconds_full" => 2,
            _ => return None,
        };

        if callback_arg_index >= call.arguments.len() {
            return None;
        }

        // Get the callback argument (should be an identifier)
        let arg_expr = call.get_arg(callback_arg_index)?;
        if let Expression::Identifier(id) = arg_expr {
            Some(&id.name)
        } else {
            None
        }
    }

    fn check_statements(
        &self,
        file_path: &std::path::Path,
        statements: &[Statement],
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        for stmt in statements {
            for ret_stmt in stmt.iter_returns() {
                if let Some(value) = &ret_stmt.value {
                    self.check_return_value(file_path, value, source, violations);
                }
            }
        }
    }

    fn check_return_value(
        &self,
        file_path: &std::path::Path,
        expr: &Expression,
        _source: &[u8],
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
