use gobject_ast::{Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct GSourceIdNotStored;

impl Rule for GSourceIdNotStored {
    fn name(&self) -> &'static str {
        "g_source_id_not_stored"
    }

    fn description(&self) -> &'static str {
        "Warn when GSource timeout/idle functions are called without storing the returned ID"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Suspicious
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // List of GSource functions that return a source ID
        const SOURCE_FUNCTIONS: &[&str] = &[
            "g_timeout_add",
            "g_timeout_add_full",
            "g_timeout_add_seconds",
            "g_timeout_add_seconds_full",
            "g_timeout_add_once",
            "g_timeout_add_seconds_once",
            "g_idle_add",
            "g_idle_add_full",
            "g_idle_add_once",
        ];

        // Walk all statements (including nested) and check expression statements
        for stmt in &func.body_statements {
            stmt.walk(&mut |s| {
                // Only check expression statements (not assignments/declarations)
                if let Statement::Expression(expr_stmt) = s
                    && expr_stmt.is_call_to_any(SOURCE_FUNCTIONS)
                    && let Expression::Call(call) = expr_stmt.as_ref() {
                        // Check if user_data (last argument) is not NULL
                        if !call.arguments.is_empty()
                            && call.has_arg_matching(call.arguments.len() - 1, |expr| {
                                !expr.is_null()
                            })
                        {
                            violations.push(self.violation(
                                &file.path,
                                call.location.line,
                                call.location.column,
                                format!(
                                    "{}() called without storing the returned source ID. If the object is destroyed before the callback fires, this will cause a use-after-free. Store the ID and use g_clear_handle_id() in dispose.",
                                    call.function_name(&file.source)
                                ),
                            ));
                        }
                    }
            });
        }
    }
}
