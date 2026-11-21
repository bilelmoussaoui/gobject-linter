use gobject_ast::model::{
    AssignmentOp, Expression, FileModel, FunctionDefItem, Statement, UnaryOp,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGSetStr;

/// The kind of value held by the variable being freed
#[derive(Debug, Clone, Copy, PartialEq)]
enum VarKind {
    Str,
    Strv,
    DateTime,
}

impl VarKind {
    /// The function used to free the old value
    fn free_fn(self) -> &'static str {
        match self {
            Self::Str => "g_free",
            Self::Strv => "g_strfreev",
            Self::DateTime => "g_date_time_unref",
        }
    }

    /// Setter that duplicates the new value
    fn copy_setter(self) -> &'static str {
        match self {
            Self::Str => "g_set_str",
            Self::Strv => "g_set_strv",
            Self::DateTime => "g_set_date_time",
        }
    }

    /// Setter that takes ownership of the new value
    fn take_setter(self) -> Option<&'static str> {
        match self {
            Self::Str => Some("g_set_str_take"),
            Self::Strv => Some("g_set_strv_take"),
            Self::DateTime => None,
        }
    }

    /// Function whose result the copy setter duplicates
    fn dup_fn(self) -> &'static str {
        match self {
            Self::Str => "g_strdup",
            Self::Strv => "g_strdupv",
            Self::DateTime => "g_date_time_ref",
        }
    }
}

impl Rule for UseGSetStr {
    fn name(&self) -> &'static str {
        "use_g_set_str"
    }

    fn description(&self) -> &'static str {
        "Suggest g_set_str()/g_set_str_take()/g_set_strv()/g_set_strv_take()/g_set_date_time() instead of manual free and assignment"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/use_g_set_str.md"))
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
            self.try_replace_pair(s1, s2, file, config, violations);
        });
    }
}

impl UseGSetStr {
    fn try_replace_pair(
        &self,
        s1: &Statement,
        s2: &Statement,
        file: &FileModel,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let Some((var, kind)) = self.extract_freed_var(s1) else {
            return;
        };
        let Some((setter, new_val, assign_desc)) = self.extract_assignment(var, kind, s2) else {
            return;
        };

        // Only g_set_str() is available before GLib 2.90
        if setter != "g_set_str" && !config.glib_at_least(2, 90) {
            return;
        }

        let var_name = var.location().as_str().unwrap_or_default();
        let replacement = config
            .style
            .format_addr_call_stmt(setter, var_name, &[new_val.as_str()]);
        let message = format!(
            "Use {replacement} instead of {} and {assign_desc}",
            kind.free_fn()
        );
        // Use two separate fixes to preserve comments between statements
        let s2_end = s2.location().find_semicolon_end();
        let fixes = vec![
            // Delete the entire first line (the free call)
            Fix::delete_line(s1.location()),
            // Replace the second statement with the setter call
            Fix::new(s2.location().start_byte, s2_end, replacement),
        ];

        violations.push(self.violation_with_fixes_at(&file.path, s1.location(), message, fixes));
    }

    /// Extract the variable from g_free(var), g_strfreev(var),
    /// g_date_time_unref(var) or g_clear_pointer(&var, <free function>)
    fn extract_freed_var<'a>(&self, stmt: &'a Statement) -> Option<(&'a Expression, VarKind)> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };
        let Expression::Call(call) = expr_stmt.as_ref() else {
            return None;
        };

        let (kind, var) = if call.is_function("g_free") {
            (VarKind::Str, call.get_arg(0)?)
        } else if call.is_function("g_strfreev") {
            (VarKind::Strv, call.get_arg(0)?)
        } else if call.is_function("g_date_time_unref") {
            (VarKind::DateTime, call.get_arg(0)?)
        } else if call.is_function("g_clear_pointer") && call.arguments.len() == 2 {
            let destroy = match call.get_arg(1)? {
                Expression::Identifier(id) => id.name.as_str(),
                _ => return None,
            };
            let kind = match destroy {
                "g_free" => VarKind::Str,
                "g_strfreev" => VarKind::Strv,
                "g_date_time_unref" => VarKind::DateTime,
                _ => return None,
            };
            // First argument must be &var
            let Expression::Unary(unary) = call.get_arg(0)? else {
                return None;
            };
            if unary.operator != UnaryOp::AddressOf {
                return None;
            }
            (kind, &*unary.operand)
        } else {
            return None;
        };

        Some((var, kind))
    }

    /// Extract (setter, new value, assignment description) from `var = rhs`
    fn extract_assignment(
        &self,
        var: &Expression,
        kind: VarKind,
        stmt: &Statement,
    ) -> Option<(&'static str, String, &'static str)> {
        let Statement::Expression(expr_stmt) = stmt else {
            return None;
        };
        let Expression::Assignment(assign) = expr_stmt.as_ref() else {
            return None;
        };
        if assign.operator != AssignmentOp::Assign {
            return None;
        }
        if assign.lhs.as_ref() != var {
            return None;
        }

        // Copy: var = g_strdup(x) and friends
        if let Expression::Call(call) = &*assign.rhs
            && call.is_function(kind.dup_fn())
            && !call.arguments.is_empty()
        {
            let new_val = call.get_arg(0)?.location().as_str()?;
            return Some((kind.copy_setter(), new_val.to_string(), kind.dup_fn()));
        }

        // var = cond ? g_strdup(cond) : NULL is equivalent to
        // g_set_str(&var, cond), which handles a NULL value
        if kind == VarKind::Str
            && let Expression::Conditional(cond) = &*assign.rhs
            && let Expression::Call(call) = &*cond.then_expr
            && call.is_function("g_strdup")
            && matches!(&*cond.else_expr, Expression::Null(_))
            && call
                .get_arg(0)
                .and_then(|arg| arg.location().as_str())
                .is_some_and(|arg| arg == cond.condition.location().as_str().unwrap_or_default())
        {
            let cond_text = cond.condition.location().as_str()?;
            return Some(("g_set_str", cond_text.to_string(), "g_strdup"));
        }

        // Ownership transfer: var = g_steal_pointer(&x) or an allocating call
        if let Some(setter) = kind.take_setter()
            && let Some(new_val) = self.extract_owned_value(kind, &assign.rhs)
        {
            return Some((setter, new_val, "an ownership-taking assignment"));
        }

        None
    }

    /// Extract the value text from an expression that transfers ownership of
    /// its result: g_steal_pointer(&x) or an allocating call
    fn extract_owned_value(&self, kind: VarKind, rhs: &Expression) -> Option<String> {
        let Expression::Call(call) = rhs else {
            return None;
        };

        if call.is_function("g_steal_pointer") && call.arguments.len() == 1 {
            // The setter takes ownership itself, no need to keep the wrapper
            let Expression::Unary(unary) = call.get_arg(0)? else {
                return None;
            };
            if unary.operator != UnaryOp::AddressOf {
                return None;
            }
            return unary.operand.location().as_str().map(str::to_string);
        }

        if call.is_allocation_call() && !call.is_function(kind.dup_fn()) {
            return rhs.location().as_str().map(str::to_string);
        }

        None
    }
}
