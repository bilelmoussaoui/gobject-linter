use gobject_ast::model::{Expression, FileModel, FunctionDefItem, SourceLocation, Statement};

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
        _ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_statements(
            file,
            func,
            &func.body_statements,
            &file.source,
            &config.style,
            violations,
        );
    }
}

impl GTaskSourceTag {
    fn check_statements(
        &self,
        file: &FileModel,
        func: &FunctionDefItem,
        statements: &[Statement],
        source: &[u8],
        style: &crate::config::Style,
        violations: &mut Vec<Violation>,
    ) {
        // Find all g_task_new calls and their variables
        let task_vars = self.find_gtask_new_vars(statements, source);

        // For each task variable, check if there's a set_source_tag call
        for (var_name, name_location, stmt_location) in task_vars {
            if !self.has_set_source_tag_call(statements, var_name, source) {
                // Extract indentation from the statement
                let indentation = stmt_location.extract_indentation(source);

                let stmt_end = stmt_location.find_semicolon_end(source);
                let call = style.format_call_stmt("g_task_set_source_tag", &[var_name, &func.name]);
                let fix = Fix::new(stmt_end, stmt_end, format!("\n{}{}", indentation, call));

                violations.push(self.violation_with_fix(
                    &file.path,
                    name_location.line,
                    name_location.column,
                    format!("GTask '{}' created without g_task_set_source_tag", var_name),
                    fix,
                ));
            }
        }
    }

    fn find_gtask_new_vars<'a>(
        &self,
        statements: &'a [Statement],
        source: &'a [u8],
    ) -> Vec<(&'a str, SourceLocation, SourceLocation)> {
        let mut results = Vec::new();

        for stmt in statements {
            stmt.walk(&mut |s| {
                match s {
                    // Check declarations: GTask *task = g_task_new(...)
                    Statement::Declaration(decl) => {
                        if let Some(Expression::Call(call)) = &decl.initializer
                            && call.is_function("g_task_new")
                        {
                            results.push((decl.name.as_str(), decl.name_location, decl.location));
                        }
                    }
                    // Check assignments: task = g_task_new(...)
                    Statement::Expression(expr_stmt) => {
                        if let Expression::Assignment(assignment) = expr_stmt.as_ref()
                            && let Expression::Call(call) = assignment.rhs.as_ref()
                            && call.is_function("g_task_new")
                        {
                            let var_name = assignment.lhs_as_text(source);
                            if !var_name.is_empty() {
                                results.push((
                                    var_name,
                                    assignment.location,
                                    *expr_stmt.location(),
                                ));
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
