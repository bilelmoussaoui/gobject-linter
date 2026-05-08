use gobject_ast::Expression;

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGValueSetStaticString;

impl Rule for UseGValueSetStaticString {
    fn name(&self) -> &'static str {
        "use_g_value_set_static_string"
    }

    fn description(&self) -> &'static str {
        "Use g_value_set_static_string for string literals instead of g_value_set_string"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Perf
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
        for call in func.find_calls(&["g_value_set_string"]) {
            self.check_call(file, call, violations);
        }
    }
}

impl UseGValueSetStaticString {
    fn check_call(
        &self,
        file: &gobject_ast::FileModel,

        call: &gobject_ast::CallExpression,
        violations: &mut Vec<Violation>,
    ) {
        // Need at least 2 arguments
        if call.arguments.len() < 2 {
            return;
        }

        // Check if second argument is a string literal
        let Some(second_expr) = call.get_arg(1) else {
            return;
        };
        if !second_expr.is_string_literal() {
            return;
        }

        // Get the string literal for the message
        let Expression::StringLiteral(string_lit) = second_expr else {
            unreachable!();
        };

        // Build the fix - replace just the function name
        let replacement = format!(
            "g_value_set_static_string ({})",
            call.arguments
                .iter()
                .filter_map(|arg| arg.to_source_string(&file.source))
                .collect::<Vec<_>>()
                .join(", ")
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
            format!(
                "Use g_value_set_static_string instead of g_value_set_string for string literal {}",
                string_lit.value
            ),
            fix,
        ));
    }
}
