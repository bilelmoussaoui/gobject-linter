use gobject_ast::model::{BinaryOp, Expression, FileModel, FunctionDefItem, UnaryOp};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct StrcmpExplicitComparison;

impl Rule for StrcmpExplicitComparison {
    fn name(&self) -> &'static str {
        "strcmp_explicit_comparison"
    }

    fn description(&self) -> &'static str {
        "Require explicit comparison with 0 for strcmp/strncmp/g_strcmp0/g_ascii_strcasecmp (returns 0 for equality, not TRUE)"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!(
            "../../docs/rules/strcmp_explicit_comparison.md"
        ))
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for stmt in &func.body_statements {
            for if_stmt in stmt.iter_if_statements() {
                self.check_condition(&if_stmt.condition, file, violations);
            }
            for while_stmt in stmt.iter_while_statements() {
                self.check_condition(&while_stmt.condition, file, violations);
            }
            for for_stmt in stmt.iter_for_statements() {
                if let Some(condition) = &for_stmt.condition {
                    self.check_condition(condition, file, violations);
                }
            }
        }
    }
}

impl StrcmpExplicitComparison {
    fn check_condition(
        &self,
        condition: &Expression,
        file: &FileModel,

        violations: &mut Vec<Violation>,
    ) {
        match condition.remove_g_likely_wrapper() {
            // Binary expression: check if it's a comparison with strcmp, or recurse for logical ops
            Expression::Binary(binary) => {
                // If it's a comparison operator, don't flag strcmp calls on either side
                // (they already have explicit comparison)
                match binary.operator {
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        // Don't recurse - strcmp calls here are OK
                    }
                    // For logical operators, recurse into both sides
                    BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                        self.check_condition(&binary.left, file, violations);
                        self.check_condition(&binary.right, file, violations);
                    }
                    _ => {
                        // For other binary operators, recurse
                        self.check_condition(&binary.left, file, violations);
                        self.check_condition(&binary.right, file, violations);
                    }
                }
            }
            // Bare call: if (strcmp(a, b)) or if (g_strcmp0(a, b))
            Expression::Call(call)
                if call
                    .function_name_str()
                    .is_some_and(|name| self.is_str_compare(name)) =>
            {
                let func_name = call.function_name();
                // Fix: add "!= 0" after the call
                let fix = Fix::new(
                    call.location.end_byte,
                    call.location.end_byte,
                    " != 0".to_string(),
                );

                violations.push(self.violation_with_fix_at(
                    &file.path,
                    &call.location,
                    format!(
                        "{}() returns 0 for equality — add explicit comparison: '{}(...) != 0'",
                        func_name, func_name
                    ),
                    fix,
                ));
            }
            // Negated call: if (!strcmp(a, b)) or if (!g_strcmp0(a, b))
            Expression::Unary(unary) if unary.operator == UnaryOp::Not => {
                if let Expression::Call(call) = &*unary.operand
                    && call
                        .function_name_str()
                        .is_some_and(|name| self.is_str_compare(name))
                {
                    let func_name = call.function_name();
                    // Fix: remove the '!' and add ' == 0' after the call
                    let fixes = vec![
                        // Remove the '!' operator
                        Fix::delete(unary.location.start_byte, call.location.start_byte),
                        // Add ' == 0' after the call
                        Fix::new(
                            call.location.end_byte,
                            call.location.end_byte,
                            " == 0".to_string(),
                        ),
                    ];

                    violations.push(self.violation_with_fixes_at(
                        &file.path,
                        &call.location,
                        format!(
                            "{}() returns 0 for equality — use '{}(...) == 0' instead of '!{}(...)'",
                            func_name, func_name, func_name
                        ),
                        fixes,
                    ));
                }
            }
            _ => {}
        }
    }

    fn is_str_compare(&self, func_name: &str) -> bool {
        matches!(
            func_name,
            "strcmp" | "strncmp" | "g_strcmp0" | "g_ascii_strcasecmp" | "g_ascii_strncasecmp"
        )
    }
}
