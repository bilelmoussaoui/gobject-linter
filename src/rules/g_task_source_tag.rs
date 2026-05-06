use gobject_ast::{Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct GTaskSourceTag;

impl Rule for GTaskSourceTag {
    fn name(&self) -> &'static str {
        "g_task_source_tag"
    }

    fn description(&self) -> &'static str {
        "Ensure g_task_set_source_tag is called after g_task_new"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Pedantic
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
        self.check_statements(path, func, &func.body_statements, source, violations);
    }
}

impl GTaskSourceTag {
    fn check_statements(
        &self,
        file_path: &std::path::Path,
        func: &gobject_ast::top_level::FunctionDefItem,
        statements: &[Statement],
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        // Find all g_task_new calls and their variables
        let task_vars = self.find_gtask_new_vars(statements);

        // For each task variable, check if there's a set_source_tag call
        for (var_name, name_location, stmt_location) in task_vars {
            if !self.has_set_source_tag_call(statements, &var_name, source) {
                // Extract indentation from the statement
                let indentation = stmt_location.extract_indentation(source);

                // Create fix: insert g_task_set_source_tag after the statement
                let fix = Fix::new(
                    stmt_location.end_byte,
                    stmt_location.end_byte,
                    format!(
                        "\n{}g_task_set_source_tag ({}, {});",
                        indentation, var_name, func.name
                    ),
                );

                violations.push(self.violation_with_fix(
                    file_path,
                    name_location.line,
                    name_location.column,
                    format!("GTask '{}' created without g_task_set_source_tag", var_name),
                    fix,
                ));
            }
        }
    }

    fn find_gtask_new_vars(
        &self,
        statements: &[Statement],
    ) -> Vec<(
        String,
        gobject_ast::SourceLocation,
        gobject_ast::SourceLocation,
    )> {
        let mut results = Vec::new();

        for stmt in statements {
            stmt.walk(&mut |s| {
                match s {
                    // Check declarations: GTask *task = g_task_new(...)
                    Statement::Declaration(decl) => {
                        if let Some(Expression::Call(call)) = &decl.initializer
                            && call.is_function("g_task_new")
                        {
                            results.push((decl.name.clone(), decl.name_location, decl.location));
                        }
                    }
                    // Check assignments: task = g_task_new(...)
                    Statement::Expression(expr_stmt) => {
                        if let Expression::Assignment(assignment) = &expr_stmt.expr
                            && let Expression::Call(call) = assignment.rhs.as_ref()
                            && call.is_function("g_task_new")
                        {
                            // For assignments, use assignment location for name, expr_stmt location
                            // for statement (expr_stmt.location
                            // includes the semicolon, assignment.location does not)
                            let var_name = assignment.lhs_as_text();
                            if !var_name.is_empty() {
                                results.push((var_name, assignment.location, expr_stmt.location));
                            }
                        }
                    }
                    _ => {}
                }
            });
        }

        results
    }

    fn has_set_source_tag_call(
        &self,
        statements: &[Statement],
        var_name: &str,
        source: &[u8],
    ) -> bool {
        for stmt in statements {
            let mut found = false;
            stmt.walk(&mut |s| {
                if let Some(call) = s.extract_call()
                    && call.is_function("g_task_set_source_tag")
                    && !call.arguments.is_empty()
                {
                    // Check if first argument contains our variable
                    if let Some(arg_text) = call.get_arg_text(0, source)
                        && arg_text.contains(var_name)
                    {
                        found = true;
                    }
                }
            });
            if found {
                return true;
            }
        }
        false
    }
}
