use gobject_ast::Expression;

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGStringFreeAndSteal;

impl Rule for UseGStringFreeAndSteal {
    fn name(&self) -> &'static str {
        "use_g_string_free_and_steal"
    }

    fn description(&self) -> &'static str {
        "Suggest g_string_free_and_steal instead of g_string_free (..., FALSE) for better readability"
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
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for call in func.find_calls(&["g_string_free"]) {
            self.check_call(file, call, violations);
        }
    }
}

impl UseGStringFreeAndSteal {
    fn check_call(
        &self,
        file: &gobject_ast::FileModel,
        call: &gobject_ast::CallExpression,
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() != 2 {
            return;
        }

        // Check if second argument is FALSE/false/0
        let Some(second_expr) = call.get_arg(1) else {
            return;
        };
        let is_false = match second_expr {
            Expression::Boolean(b) => !b.value,
            Expression::NumberLiteral(n) => n.value == "0",
            _ => false,
        };

        if !is_false {
            return;
        }

        // Get argument text for the fix
        let Some(first_text) = call.get_arg_text(0, &file.source) else {
            return;
        };
        let Some(second_text) = call.get_arg_text(1, &file.source) else {
            return;
        };

        // Build replacement
        let replacement = format!("g_string_free_and_steal ({})", first_text);
        let message = format!(
            "Use {} instead of g_string_free({}, {}) for readability",
            replacement, first_text, second_text
        );
        let fix = Fix::new(
            call.location.start_byte,
            call.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix(
            &file.path,
            call.location.line,
            call.location.column,
            message,
            fix,
        ));
    }
}
