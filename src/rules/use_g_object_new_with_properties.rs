use gobject_ast::{CallExpression, Expression, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct UseGObjectNewWithProperties;

impl Rule for UseGObjectNewWithProperties {
    fn name(&self) -> &'static str {
        "use_g_object_new_with_properties"
    }

    fn description(&self) -> &'static str {
        "Suggest setting properties in g_object_new instead of separate g_object_set calls"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Find all g_object_new calls with no properties
        let empty_new_calls: Vec<_> = func
            .find_calls(&["g_object_new"])
            .into_iter()
            .filter(|call| self.is_g_object_new_empty(call))
            .collect();

        if empty_new_calls.is_empty() {
            return;
        }

        // Check statements for the pattern
        self.check_statements(&func.body_statements, &empty_new_calls, file, violations);
    }
}

impl UseGObjectNewWithProperties {
    fn check_statements(
        &self,
        statements: &[Statement],
        empty_new_calls: &[&CallExpression],
        file: &gobject_ast::FileModel,

        violations: &mut Vec<Violation>,
    ) {
        for i in 0..statements.len() {
            // Check if this statement contains one of our empty g_object_new calls
            if let Some((var_name, location)) =
                self.find_empty_new_in_statement(&statements[i], empty_new_calls)
            {
                // Count consecutive g_object_set calls on the same variable
                let mut set_count = 0;

                for next_stmt in statements.iter().skip(i + 1) {
                    if let Some(set_var) = self.extract_g_object_set(next_stmt)
                        && set_var == var_name
                    {
                        set_count += 1;
                        continue;
                    }

                    // Stop if we hit something that's not a g_object_set on our variable
                    break;
                }

                // Only report if there's at least one g_object_set call
                if set_count > 0 {
                    violations.push(self.violation(
                        &file.path,
                        location.line,
                        location.column,
                        format!(
                            "Set properties in g_object_new() instead of {} separate g_object_set() call{}",
                            set_count,
                            if set_count > 1 { "s" } else { "" }
                        ),
                    ));
                }
            }

            statements[i].for_each_child_block(|body| {
                self.check_statements(body, empty_new_calls, file, violations);
            });
        }
    }

    /// Check if a statement contains one of the empty g_object_new calls
    /// Returns (variable_name, statement_location) if found
    fn find_empty_new_in_statement(
        &self,
        stmt: &Statement,
        empty_new_calls: &[&CallExpression],
    ) -> Option<(String, gobject_ast::SourceLocation)> {
        match stmt {
            // Declaration: FooObject *obj = g_object_new(TYPE, NULL);
            Statement::Declaration(decl) => {
                if let Some(Expression::Call(call)) = &decl.initializer {
                    // Check if this call is one of our empty g_object_new calls
                    for &empty_call in empty_new_calls {
                        if std::ptr::eq(call as *const _, empty_call as *const _) {
                            return Some((decl.name.clone(), decl.location));
                        }
                    }
                }
            }
            // Assignment: obj = g_object_new(TYPE, NULL);
            Statement::Expression(expr_stmt) => {
                if let Expression::Assignment(assign) = expr_stmt.as_ref()
                    && let Expression::Call(call) = &*assign.rhs
                {
                    for &empty_call in empty_new_calls {
                        if std::ptr::eq(call as *const _, empty_call as *const _) {
                            let var_name = assign.lhs_as_text();
                            if !var_name.is_empty() {
                                return Some((var_name, *expr_stmt.location()));
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }

    /// Check if a call is g_object_new with no properties (just NULL or type
    /// only)
    fn is_g_object_new_empty(&self, call: &CallExpression) -> bool {
        if !call.is_function("g_object_new") {
            return false;
        }

        // g_object_new with just type and NULL, or just type
        // g_object_new(TYPE, NULL) - 2 args
        // g_object_new(TYPE) - 1 arg (rare but valid)
        match call.arguments.len() {
            1 => true,
            2 => call.arguments[1].is_null(),
            _ => false,
        }
    }

    /// Extract g_object_set call, return the object variable
    fn extract_g_object_set(&self, stmt: &Statement) -> Option<String> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };

        let Expression::Call(call) = expr_stmt.as_ref() else {
            return None;
        };

        if !call.is_function("g_object_set") {
            return None;
        }

        // Get the first argument (the object)
        let expr = call.get_arg(0)?;
        expr.extract_variable_name()
    }
}
