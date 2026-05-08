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
        _ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_statements(file, &func.body_statements, violations);
    }
}

impl UseGClearHandleId {
    fn check_statements(
        &self,
        file: &gobject_ast::FileModel,
        statements: &[Statement],
        violations: &mut Vec<Violation>,
    ) {
        // Check the statements themselves for cleanup pattern
        self.check_compound_statement(file, statements, violations);

        // Recurse into nested statements
        for stmt in statements {
            if let Statement::If(if_stmt) = stmt {
                // check_if_statement returns true if it handled then_body itself
                let handled = self.check_if_statement(file, if_stmt, violations);
                if !handled {
                    self.check_statements(file, &if_stmt.then_body, violations);
                }
                if let Some(else_body) = &if_stmt.else_body {
                    self.check_statements(file, else_body, violations);
                }
            } else {
                stmt.for_each_child_block(|body| {
                    self.check_statements(file, body, violations);
                });
            }
        }
    }

    fn check_if_statement(
        &self,
        file: &gobject_ast::FileModel,
        if_stmt: &gobject_ast::IfStatement,
        violations: &mut Vec<Violation>,
    ) -> bool {
        let conversions = self.check_cleanup_then_zero(file, &if_stmt.then_body);

        if !conversions.is_empty() {
            let stmt_count = if_stmt.then_body.len();

            let has_else = if_stmt.else_body.is_some();
            let cond_id = if_stmt.extract_nonzero_check_variable(&file.source);

            for (var_name, cleanup_func, first_loc, second_loc) in conversions {
                let replacement = format!("g_clear_handle_id (&{}, {});", var_name, cleanup_func);
                let message = format!(
                    "Use {} instead of {} and zero assignment",
                    replacement, cleanup_func
                );
                let can_remove_if = !has_else && cond_id == Some(var_name) && stmt_count == 2;

                let fix = if can_remove_if {
                    Fix::new(
                        if_stmt.location.start_byte,
                        if_stmt.location.end_byte,
                        replacement,
                    )
                } else if stmt_count == 2 {
                    // Find braces around the statements
                    let first_start = if_stmt.then_body[0].location().start_byte;
                    let (mut brace_start, brace_end) =
                        SourceLocation::find_braces_around(first_start, &file.source);

                    // Include the newline before the brace in the replacement
                    while brace_start > 0 && file.source[brace_start - 1] != b'\n' {
                        brace_start -= 1;
                    }
                    brace_start = brace_start.saturating_sub(1);

                    // Extract indentation from the line after the brace
                    let brace_location =
                        SourceLocation::new(0, 0, brace_start + 1, brace_start + 1);
                    let indent = brace_location.extract_line_indentation(&file.source);
                    let formatted_replacement = format!("\n{}{}", indent, replacement);

                    Fix::new(brace_start, brace_end, formatted_replacement)
                } else {
                    Fix::new(first_loc.start_byte, second_loc.end_byte, replacement)
                };

                violations.push(self.violation_with_fix(
                    &file.path,
                    first_loc.line,
                    first_loc.column,
                    message,
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
            let call_text = call.location.as_str(&file.source).unwrap_or("");

            let loc = if_stmt.then_body[0].location();
            let fix = Fix::new(loc.start_byte, loc.end_byte, format!("{};", call_text));

            violations.push(self.violation_with_fix(
                &file.path,
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
        file: &gobject_ast::FileModel,
        statements: &[Statement],
        violations: &mut Vec<Violation>,
    ) {
        for (var_name, cleanup_func, first_loc, second_loc) in
            self.check_cleanup_then_zero(file, statements)
        {
            let replacement = format!("g_clear_handle_id (&{}, {});", var_name, cleanup_func);
            let message = format!(
                "Use {} instead of {} and zero assignment",
                replacement, cleanup_func
            );
            let fix = Fix::new(first_loc.start_byte, second_loc.end_byte, replacement);

            violations.push(self.violation_with_fix(
                &file.path,
                first_loc.line,
                first_loc.column,
                message,
                fix,
            ));
        }
    }

    fn check_cleanup_then_zero<'a>(
        &self,
        file: &'a gobject_ast::FileModel,
        statements: &[Statement],
    ) -> Vec<(&'a str, String, SourceLocation, SourceLocation)> {
        let mut results = Vec::new();

        Statement::for_each_pair(statements, |first, second| {
            if let Some((var_name, cleanup_func)) = self.extract_handle_cleanup(first, file)
                && second.is_assignment_to(var_name, gobject_ast::Expression::is_zero, &file.source)
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

    fn extract_handle_cleanup<'a>(
        &self,
        stmt: &Statement,
        file: &'a gobject_ast::FileModel,
    ) -> Option<(&'a str, String)> {
        let call = stmt.extract_call()?;

        let func_name = call.function_name_str()?;
        let is_handle_cleanup = matches!(func_name, "g_source_remove" | "g_source_destroy");

        if !is_handle_cleanup {
            return None;
        }

        let arg_expr = call.get_arg(0)?;
        let var_name = arg_expr.location().as_str(&file.source)?.trim();

        Some((var_name, func_name.to_owned()))
    }
}
