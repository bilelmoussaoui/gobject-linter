use gobject_ast::{BinaryOp, Expression, IfStatement, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

const UNREF_FUNCTIONS: &[&str] = &[
    "g_object_unref",
    "g_free",
    "g_hash_table_unref",
    "g_hash_table_destroy",
    "g_list_free",
    "g_slist_free",
    "g_array_unref",
    "g_bytes_unref",
    "g_variant_unref",
];

pub struct UseClearFunctions;

impl Rule for UseClearFunctions {
    fn name(&self) -> &'static str {
        "use_clear_functions"
    }

    fn description(&self) -> &'static str {
        "Suggest g_clear_object/g_clear_pointer instead of manual unref and NULL assignment"
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

impl UseClearFunctions {
    fn check_statements(
        &self,
        file_path: &std::path::Path,
        statements: &[Statement],
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) {
        // Check for consecutive unref + NULL assignment pattern
        let mut i = 0;
        while i + 1 < statements.len() {
            if self.check_consecutive_pattern(
                file_path,
                &statements[i],
                &statements[i + 1],
                source,
                violations,
            ) {
                i += 2; // Skip both statements
                continue;
            }
            i += 1;
        }

        // Check for if-statement pattern and recurse
        for stmt in statements {
            if let Statement::If(if_stmt) = stmt {
                // check_if_statement returns true if it matched the pattern itself;
                // only recurse into bodies when it didn't to avoid duplicate violations
                let matched_if = self.check_if_statement(file_path, if_stmt, source, violations);
                if !matched_if {
                    self.check_statements(file_path, &if_stmt.then_body, source, violations);
                    if let Some(else_body) = &if_stmt.else_body {
                        self.check_statements(file_path, else_body, source, violations);
                    }
                }
            } else {
                stmt.for_each_child_block(|body| {
                    self.check_statements(file_path, body, source, violations);
                });
            }
        }
    }

    fn check_consecutive_pattern(
        &self,
        file_path: &std::path::Path,
        stmt1: &Statement,
        stmt2: &Statement,
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) -> bool {
        // Check if first statement is an unref/free call
        let Some(call) = stmt1.extract_call() else {
            return false;
        };

        let mut found_function = None;
        let mut var_name = None;

        for &func_name in UNREF_FUNCTIONS {
            if call.is_function(func_name) {
                // Get the first argument
                if let Some(arg) = call.get_arg(0)
                    && let Some(arg_text) = arg.to_source_string(source)
                {
                    var_name = Some(arg_text);
                    found_function = Some(func_name);
                    break;
                }
            }
        }

        let Some(var) = var_name else {
            return false;
        };
        let Some(unref_func) = found_function else {
            return false;
        };

        // Check if second statement is NULL assignment to the same variable
        if !stmt2.is_assignment_to(&var, |expr| expr.is_null() || expr.is_zero()) {
            return false;
        }

        // Build the replacement
        let suggested_function = self.suggest_clear_function(unref_func);
        let replacement = if suggested_function == "g_clear_object" {
            format!("g_clear_object (&{});", var)
        } else {
            format!("g_clear_pointer (&{}, {});", var, unref_func)
        };

        // Create fixes: replace first statement, delete second
        let stmt1_end = stmt1.location().find_semicolon_end(source);
        let fixes = vec![
            Fix::new(stmt1.location().start_byte, stmt1_end, replacement.clone()),
            Fix::delete_line(stmt2.location(), source),
        ];

        violations.push(self.violation_with_fixes(
            file_path,
            stmt1.location().line,
            stmt1.location().column,
            format!(
                "Use {} instead of manual unref and NULL assignment",
                replacement
            ),
            fixes,
        ));

        true
    }

    fn check_if_statement(
        &self,
        file_path: &std::path::Path,
        if_stmt: &IfStatement,
        source: &[u8],
        violations: &mut Vec<Violation>,
    ) -> bool {
        // Check if condition has && or || operators - if so, skip
        // g_clear_pointer only checks NULL, not other conditions
        if self.has_logical_operators(&if_stmt.condition) {
            return false;
        }

        // Get the variable being checked in the condition
        let Some(checked_var) = self.find_variable_in_condition(&if_stmt.condition, source) else {
            return false;
        };

        // Check if body has exactly 2 statements
        if if_stmt.then_body.len() != 2 {
            return false;
        }

        // Look for unref/free call and NULL assignment
        let Some((unref_function, _unref_stmt)) =
            self.find_unref_call(&if_stmt.then_body, &checked_var, source)
        else {
            return false;
        };

        if !self.has_null_assignment(&if_stmt.then_body, &checked_var) {
            return false;
        }

        // Build the replacement
        let suggested_function = self.suggest_clear_function(&unref_function);
        let replacement = if suggested_function == "g_clear_object" {
            format!("g_clear_object (&{});", checked_var)
        } else {
            format!("g_clear_pointer (&{}, {});", checked_var, unref_function)
        };

        let fix = Fix::new(
            if_stmt.location.start_byte,
            if_stmt.location.end_byte,
            replacement.clone(),
        );

        violations.push(self.violation_with_fix(
            file_path,
            if_stmt.location.line,
            if_stmt.location.column,
            format!(
                "Use {} instead of manual NULL check, unref, and assignment",
                replacement
            ),
            fix,
        ));

        true
    }

    fn find_variable_in_condition(&self, expr: &Expression, source: &[u8]) -> Option<String> {
        // Try direct variable extraction first
        if let Some(var) = expr.extract_variable_name() {
            return Some(var);
        }

        match expr {
            Expression::Binary(bin) => {
                // For binary expressions (field != NULL), try left side first
                if let Some(var) = self.find_variable_in_condition(&bin.left, source) {
                    return Some(var);
                }
                // Then try right side
                self.find_variable_in_condition(&bin.right, source)
            }
            Expression::Unary(unary) => {
                // For unary expressions like (!ptr), check the operand
                self.find_variable_in_condition(&unary.operand, source)
            }
            // For any other expression type, try to get its source text
            _ => expr.to_source_string(source),
        }
    }

    fn has_logical_operators(&self, expr: &Expression) -> bool {
        let mut found = false;
        expr.walk(&mut |e| {
            if let Expression::Binary(bin) = e
                && matches!(bin.operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr)
            {
                found = true;
            }
        });
        found
    }

    fn find_unref_call<'a>(
        &self,
        statements: &'a [Statement],
        var_name: &str,
        source: &[u8],
    ) -> Option<(String, &'a Statement)> {
        for stmt in statements {
            if let Some(call) = stmt.extract_call() {
                for &func_name in UNREF_FUNCTIONS {
                    if call.is_function(func_name) {
                        // Check if any argument contains the variable
                        for arg in &call.arguments {
                            if let Some(arg_text) = arg.to_source_string(source)
                                && arg_text.contains(var_name)
                            {
                                return Some((func_name.to_string(), stmt));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn has_null_assignment(&self, statements: &[Statement], var_name: &str) -> bool {
        statements
            .iter()
            .any(|stmt| stmt.is_assignment_to(var_name, |expr| expr.is_null() || expr.is_zero()))
    }

    fn suggest_clear_function(&self, unref_function: &str) -> &str {
        match unref_function {
            "g_object_unref" => "g_clear_object",
            "g_free" => "g_clear_pointer",
            "g_hash_table_unref" | "g_hash_table_destroy" => "g_clear_pointer",
            "g_list_free" | "g_slist_free" => "g_clear_pointer",
            "g_array_unref" => "g_clear_pointer",
            "g_bytes_unref" => "g_clear_pointer",
            "g_variant_unref" => "g_clear_pointer",
            _ => "g_clear_pointer",
        }
    }
}
