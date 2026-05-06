use gobject_ast::{Expression, SourceLocation, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGClearHandleId;

impl Rule for UseGClearHandleId {
    fn name(&self) -> &'static str {
        "use_g_clear_handle_id"
    }

    fn description(&self) -> &'static str {
        "Suggest g_clear_handle_id instead of manual cleanup and zero assignment"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        path: &std::path::Path,
        violations: &mut Vec<Violation>,
    ) {
        let source = &ast_context.project.files.get(path).unwrap().source;
        self.check_statements(path, &func.body_statements, source, violations);
    }
}

impl UseGClearHandleId {
    fn check_statements(
        &self,
        file_path: &std::path::Path,
        statements: &[Statement],
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        // Check the statements themselves for cleanup pattern
        self.check_compound_statement(file_path, statements, source, violations);

        // Recurse into nested statements
        for stmt in statements {
            if let Statement::If(if_stmt) = stmt {
                // check_if_statement returns true if it handled then_body itself
                let handled = self.check_if_statement(file_path, if_stmt, source, violations);
                if !handled {
                    self.check_statements(file_path, &if_stmt.then_body, source, violations);
                }
                if let Some(else_body) = &if_stmt.else_body {
                    self.check_statements(file_path, else_body, source, violations);
                }
            } else {
                stmt.for_each_child_block(|body| {
                    self.check_statements(file_path, body, source, violations);
                });
            }
        }
    }

    fn check_if_statement(
        &self,
        file_path: &std::path::Path,
        if_stmt: &gobject_ast::IfStatement,
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) -> bool {
        let conversions = self.check_cleanup_then_zero(&if_stmt.then_body, source);

        if !conversions.is_empty() {
            let stmt_count = if_stmt.then_body.len();

            let has_else = if_stmt.else_body.is_some();
            let cond_id = if_stmt.extract_nonzero_check_variable();

            for (var_name, cleanup_func, first_loc, second_loc) in conversions {
                let replacement = format!("g_clear_handle_id (&{}, {});", var_name, cleanup_func);

                let can_remove_if =
                    !has_else && cond_id.as_deref() == Some(var_name.as_str()) && stmt_count == 2;

                let fix = if can_remove_if {
                    Fix::new(
                        if_stmt.location.start_byte,
                        if_stmt.location.end_byte,
                        replacement.clone(),
                    )
                } else if stmt_count == 2 {
                    // Find braces around the statements
                    let first_start = if_stmt.then_body[0].location().start_byte;
                    let (mut brace_start, brace_end) =
                        SourceLocation::find_braces_around(first_start, source);

                    // Include the newline before the brace in the replacement
                    while brace_start > 0 && source[brace_start - 1] != b'\n' {
                        brace_start -= 1;
                    }
                    brace_start = brace_start.saturating_sub(1);

                    // Extract indentation from the line after the brace
                    let brace_location =
                        SourceLocation::new(0, 0, brace_start + 1, brace_start + 1);
                    let indent = brace_location.extract_line_indentation(source);
                    let formatted_replacement = format!("\n{}{}", indent, replacement);

                    Fix::new(brace_start, brace_end, formatted_replacement)
                } else {
                    Fix::new(
                        first_loc.start_byte,
                        second_loc.end_byte,
                        replacement.clone(),
                    )
                };

                violations.push(self.violation_with_fix(
                    file_path,
                    first_loc.line,
                    first_loc.column,
                    format!(
                        "Use {} instead of {} and zero assignment",
                        replacement, cleanup_func
                    ),
                    fix,
                ));
            }
            // We handled the cleanup pattern, return true to prevent double-checking
            return true;
        } else if if_stmt.then_body.len() == 1
            && if_stmt.then_has_braces
            && let Statement::Expression(expr_stmt) = &if_stmt.then_body[0]
            && let Expression::Call(call) = expr_stmt.as_ref()
            && call.is_function("g_clear_handle_id")
        {
            let call_text = call.location.as_str(source).unwrap_or("");

            let loc = if_stmt.then_body[0].location();
            let fix = Fix::new(loc.start_byte, loc.end_byte, format!("{};", call_text));

            violations.push(self.violation_with_fix(
                file_path,
                if_stmt.location.line,
                if_stmt.location.column,
                "Remove unnecessary braces around single g_clear_handle_id call".to_string(),
                fix,
            ));
        }

        // Didn't find/handle cleanup pattern, let caller recurse into then_body
        false
    }

    fn check_compound_statement(
        &self,
        file_path: &std::path::Path,
        statements: &[Statement],
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        for (var_name, cleanup_func, first_loc, second_loc) in
            self.check_cleanup_then_zero(statements, source)
        {
            let replacement = format!("g_clear_handle_id (&{}, {});", var_name, cleanup_func);

            let fix = Fix::new(
                first_loc.start_byte,
                second_loc.end_byte,
                replacement.clone(),
            );

            violations.push(self.violation_with_fix(
                file_path,
                first_loc.line,
                first_loc.column,
                format!(
                    "Use {} instead of {} and zero assignment",
                    replacement, cleanup_func
                ),
                fix,
            ));
        }
    }

    fn check_cleanup_then_zero(
        &self,
        statements: &[Statement],
        source: &[u8],
    ) -> Vec<(String, String, SourceLocation, SourceLocation)> {
        let mut results = Vec::new();

        Statement::for_each_pair(statements, |first, second| {
            if let Some((var_name, cleanup_func)) = self.extract_handle_cleanup(first, source)
                && second.is_assignment_to(&var_name, gobject_ast::Expression::is_zero)
            {
                results.push((
                    var_name,
                    cleanup_func,
                    *first.location(),
                    *second.location(),
                ));
            }
        });

        results
    }

    fn extract_handle_cleanup(&self, stmt: &Statement, source: &[u8]) -> Option<(String, String)> {
        let call = stmt.extract_call()?;

        let func_name = call.function_name_str()?;
        let is_handle_cleanup = matches!(func_name, "g_source_remove" | "g_source_destroy");

        if !is_handle_cleanup {
            return None;
        }

        let arg_expr = call.get_arg(0)?;
        let var_name = arg_expr.location().as_str(source)?.trim().to_string();

        Some((var_name, func_name.to_string()))
    }
}
