use gobject_ast::model::{
    AssignmentOp, Expression, FileModel, FunctionDefItem, Statement, UnaryOp,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGSetStr;

impl Rule for UseGSetStr {
    fn name(&self) -> &'static str {
        "use_g_set_str"
    }

    fn description(&self) -> &'static str {
        "Suggest g_set_str() instead of manual g_free and g_strdup"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 76))
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        Statement::walk_pairs(&func.body_statements, &mut |s1, s2| {
            self.try_free_then_strdup(s1, s2, file, config, violations);
        });
    }
}

impl UseGSetStr {
    fn try_free_then_strdup(
        &self,
        s1: &Statement,
        s2: &Statement,
        file: &FileModel,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) -> bool {
        // First statement: g_free(var) or g_clear_pointer(&var, g_free)
        let Some(var_name) = self.extract_gfree_var(s1) else {
            return false;
        };

        // Second statement: var = g_strdup(...)
        let Some((assign_var, new_val)) = self.extract_strdup_assignment(s2) else {
            return false;
        };

        if assign_var != var_name {
            return false;
        }

        let replacement = config
            .style
            .format_addr_call_stmt("g_set_str", var_name, &[new_val]);
        let message = format!("Use {replacement} instead of g_free and g_strdup");
        // Use two separate fixes to preserve comments between statements
        let s2_end = s2.location().find_semicolon_end();
        let fixes = vec![
            // Delete the entire first line (g_free/g_clear_pointer)
            Fix::delete_line(s1.location()),
            // Replace the second statement with g_set_str
            Fix::new(s2.location().start_byte, s2_end, replacement),
        ];

        violations.push(self.violation_with_fixes_at(&file.path, s1.location(), message, fixes));
        true
    }

    /// Extract variable from g_free(var) or g_clear_pointer(&var, g_free)
    fn extract_gfree_var<'a>(&'a self, stmt: &'a Statement) -> Option<&'a str> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };

        let Expression::Call(call) = expr_stmt.as_ref() else {
            return None;
        };

        if call.is_function("g_free") {
            return call.get_arg(0)?.location().as_str();
        } else if call.is_function("g_clear_pointer") {
            // g_clear_pointer(&var, g_free)
            if call.arguments.len() != 2 {
                return None;
            }

            // Check if second argument is g_free
            let second_arg = call.get_arg(1)?;

            if let Expression::Identifier(id) = second_arg {
                if id.name != "g_free" {
                    return None;
                }
            } else {
                return None;
            }

            // First argument is &var - extract var
            let first_arg = call.get_arg(0)?;
            if let Expression::Unary(unary) = first_arg
                && unary.operator == UnaryOp::AddressOf
            {
                return unary.operand.location().as_str();
            }
        }

        None
    }

    /// Extract (var, new_val) from var = g_strdup(new_val) or var = cond ?
    /// g_strdup(...) : NULL
    fn extract_strdup_assignment<'a>(&self, stmt: &'a Statement) -> Option<(&'a str, &'a str)> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };

        let Expression::Assignment(assign) = expr_stmt.as_ref() else {
            return None;
        };

        if assign.operator != AssignmentOp::Assign {
            return None;
        }

        // Direct g_strdup call: var = g_strdup(new_val)
        if let Expression::Call(call) = &*assign.rhs
            && call.is_function("g_strdup")
            && !call.arguments.is_empty()
        {
            let new_val = call.get_arg(0)?.location().as_str()?;
            let var_name = assign.lhs_as_text();
            if !var_name.is_empty() {
                return Some((var_name, new_val));
            }
        }

        // Ternary: var = cond ? g_strdup(...) : NULL
        if let Expression::Conditional(cond) = &*assign.rhs
            && cond.then_expr.is_call_to_any(&["g_strdup", "g_strndup"])
        {
            // Use the condition variable as the value
            let cond_text = cond.condition.location().as_str()?;
            let var_name = assign.lhs_as_text();
            if !var_name.is_empty() {
                return Some((var_name, cond_text));
            }
        }

        None
    }
}
