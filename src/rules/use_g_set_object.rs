use gobject_ast::model::{
    AssignmentOp, Expression, FileModel, FunctionDefItem, Statement, UnaryOp,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGSetObject;

impl Rule for UseGSetObject {
    fn name(&self) -> &'static str {
        "use_g_set_object"
    }

    fn description(&self) -> &'static str {
        "Suggest g_set_object() instead of manual g_clear_object and g_object_ref"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 44))
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
            self.try_clear_then_ref(s1, s2, file, config, violations);
        });
    }
}

impl UseGSetObject {
    fn try_clear_then_ref(
        &self,
        s1: &Statement,
        s2: &Statement,
        file: &FileModel,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) -> bool {
        // First statement: g_clear_object(&var) or g_object_unref(var)
        let Some((var_name, needs_deref)) = self.extract_clear_or_unref_var(s1, &file.source)
        else {
            return false;
        };

        // Second statement: var = g_object_ref(...) or *var = g_object_ref(...)
        let Some((assign_var, new_val)) = self.extract_object_ref_assignment(s2, &file.source)
        else {
            return false;
        };

        let matches = if needs_deref {
            assign_var.len() == var_name.len() + 1
                && assign_var.starts_with('*')
                && assign_var[1..] == *var_name
        } else {
            assign_var == var_name
        };
        if !matches {
            return false;
        }

        // g_set_object takes GObject**, so:
        // - If var is GObject* (needs_deref=false), use &var
        // - If var is GObject** (needs_deref=true), use var directly
        let replacement = if needs_deref {
            config
                .style
                .format_call_stmt("g_set_object", &[var_name, new_val])
        } else {
            config
                .style
                .format_addr_call_stmt("g_set_object", var_name, &[new_val])
        };

        // Use two separate fixes to preserve comments between statements
        let s2_end = s2.location().find_semicolon_end(&file.source);
        let message = format!("Use {replacement} instead of g_clear_object and g_object_ref");
        let fixes = vec![
            // Delete the entire first line (g_clear_object/g_object_unref)
            Fix::delete_line(s1.location(), &file.source),
            // Replace the second statement with g_set_object
            Fix::new(s2.location().start_byte, s2_end, replacement),
        ];

        violations.push(self.violation_with_fixes(
            &file.path,
            s1.location().line,
            s1.location().column,
            message,
            fixes,
        ));
        true
    }

    /// Extract variable from g_clear_object(&var)/g_clear_object(ptr) or
    /// g_object_unref(var) Returns (var_name, needs_deref) where
    /// needs_deref indicates if assignment should use *var
    fn extract_clear_or_unref_var<'a>(
        &self,
        stmt: &Statement,
        source: &'a [u8],
    ) -> Option<(&'a str, bool)> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };

        let Expression::Call(call) = expr_stmt.as_ref() else {
            return None;
        };

        if call.arguments.is_empty() {
            return None;
        }

        if call.is_function("g_clear_object") {
            // g_clear_object can take:
            // 1. &var - then assignment is var = ...
            // 2. ptr - then assignment is *ptr = ...
            let first_arg = call.get_arg(0)?;
            if let Expression::Unary(unary) = first_arg
                && unary.operator == UnaryOp::AddressOf
            {
                // Case 1: g_clear_object(&var)
                return Some((unary.operand.to_source_string(source)?, false));
            } else {
                // Case 2: g_clear_object(ptr) where ptr is GObject**
                return Some((first_arg.to_source_string(source)?, true));
            }
        } else if call.is_function("g_object_unref") {
            // g_object_unref(var) - assignment is var = ...
            let first_arg = call.get_arg(0)?;
            return Some((first_arg.to_source_string(source)?, false));
        }

        None
    }

    /// Extract (var, new_val) from var = g_object_ref(new_val)
    fn extract_object_ref_assignment<'a>(
        &self,
        stmt: &Statement,
        source: &'a [u8],
    ) -> Option<(&'a str, &'a str)> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };

        let Expression::Assignment(assign) = expr_stmt.as_ref() else {
            return None;
        };

        if assign.operator != AssignmentOp::Assign {
            return None;
        }

        // var = g_object_ref(new_val)
        if let Expression::Call(call) = &*assign.rhs
            && call.is_function("g_object_ref")
            && !call.arguments.is_empty()
        {
            let new_val = call.get_arg(0)?.to_source_string(source)?;
            let var_name = assign.lhs_as_text(source);
            if !var_name.is_empty() {
                return Some((var_name, new_val));
            }
        }

        None
    }
}
