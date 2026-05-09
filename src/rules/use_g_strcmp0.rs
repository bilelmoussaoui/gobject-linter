use gobject_ast::model::{Expression, FileModel, FunctionDefItem, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGStrcmp0;

impl Rule for UseGStrcmp0 {
    fn name(&self) -> &'static str {
        "use_g_strcmp0"
    }

    fn description(&self) -> &'static str {
        "Suggest g_strcmp0 instead of strcmp if arguments can be NULL (NULL-safe)"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
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
        self.check_statements(&func.body_statements, file, violations);
    }
}

impl UseGStrcmp0 {
    fn check_statements(
        &self,
        statements: &[Statement],
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for stmt in statements {
            // Walk all expressions in the statement tree (recursively)
            // walk_expressions visits each expression in the statement tree,
            // but does not recurse into nested expressions within those expressions
            stmt.walk_expressions(&mut |expr| {
                // Walk this expression and all its nested expressions
                expr.walk(&mut |e| {
                    self.check_expression(e, file, violations);
                });
            });
        }
    }

    fn check_expression(
        &self,
        expr: &Expression,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Check for strcmp usage (suggest g_strcmp0 for NULL-safety)
        if expr.is_call_to("strcmp") {
            let Expression::Call(call) = expr else {
                return;
            };
            // Create fix to replace "strcmp" with "g_strcmp0"
            let fix = Fix::new(
                call.location.start_byte,
                call.location.start_byte + "strcmp".len(),
                "g_strcmp0".to_string(),
            );

            violations.push(self.violation_with_fix(
                    &file.path,
                    call.location.line,
                    call.location.column,
                    "Consider g_strcmp0 instead of strcmp if arguments can be NULL (g_strcmp0 is NULL-safe)".to_string(),
                    fix,
                ));
        }
    }
}
