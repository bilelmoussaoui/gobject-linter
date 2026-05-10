use gobject_ast::model::{CallExpression, Expression, FileModel, FunctionDefItem};

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
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for call in func.find_calls(&["g_value_set_string"]) {
            self.check_call(file, call, config, violations);
        }
    }
}

impl UseGValueSetStaticString {
    fn check_call(
        &self,
        file: &FileModel,
        call: &CallExpression,
        config: &Config,
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
        let args: Vec<&str> = call
            .arguments
            .iter()
            .filter_map(|arg| arg.to_source_string(&file.source))
            .collect();
        let replacement = config.style.format_call("g_value_set_static_string", &args);

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
