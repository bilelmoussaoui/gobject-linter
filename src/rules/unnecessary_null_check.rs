use gobject_ast::{Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UnnecessaryNullCheck;

impl Rule for UnnecessaryNullCheck {
    fn name(&self) -> &'static str {
        "unnecessary_null_check"
    }

    fn description(&self) -> &'static str {
        "Detect unnecessary NULL checks before g_free/g_clear_* functions"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Suspicious
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
        // Walk through function body looking for if statements

        for stmt in &func.body_statements {
            for if_stmt in stmt.iter_if_statements() {
                self.check_if_statement(if_stmt, path, source, violations);
            }
        }
    }
}

impl UnnecessaryNullCheck {
    fn check_if_statement(
        &self,
        if_stmt: &gobject_ast::IfStatement,
        file_path: &std::path::Path,
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        // Don't flag if there's an else branch — removing the if would also drop the
        // else logic
        if if_stmt.has_else() {
            return;
        }

        // Extract variable being checked (e.g., "ptr" from "ptr != NULL")
        let Some(checked_var) = if_stmt.extract_null_check_variable() else {
            return;
        };

        // Check if the body contains only a g_free/g_clear_* call with the checked
        // variable
        if !if_stmt.has_single_statement() {
            return;
        }

        // Get the single statement in the then body
        let Statement::Expression(expr_stmt) = &if_stmt.then_body[0] else {
            return;
        };

        // Check if it's a g_free/g_clear_* call
        let Expression::Call(call) = expr_stmt.as_ref() else {
            return;
        };

        // Check for g_free or any g_clear_* function
        let Some(func_name) = call.function_name_str() else {
            return;
        };
        if !func_name.starts_with("g_free") && !func_name.starts_with("g_clear_") {
            return;
        }

        // Check if the call arguments reference the checked variable
        let references_var = call
            .arguments
            .iter()
            .any(|gobject_ast::Argument::Expression(e)| e.contains_identifier(&checked_var));

        if !references_var {
            return;
        }

        // Create a fix: replace the if statement with the call statement
        // Extract the statement text from the source (including the semicolon)
        let loc = expr_stmt.location();
        let stmt_end = loc.find_semicolon_end(source);
        let stmt_text = std::str::from_utf8(&source[loc.start_byte..stmt_end]).unwrap_or_default();

        let fix = Fix::new(
            if_stmt.location.start_byte,
            if_stmt.location.end_byte,
            stmt_text.to_string(),
        );

        violations.push(self.violation_with_fix(
            file_path,
            if_stmt.location.line,
            if_stmt.location.column,
            format!(
                "Remove unnecessary NULL check before {} ({} handles NULL)",
                func_name, func_name
            ),
            fix,
        ));
    }
}
