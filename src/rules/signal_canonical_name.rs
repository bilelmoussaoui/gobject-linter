use gobject_ast::{Argument, Expression};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct SignalCanonicalName;

impl Rule for SignalCanonicalName {
    fn name(&self) -> &'static str {
        "signal_canonical_name"
    }

    fn description(&self) -> &'static str {
        "Signal names should use hyphens (-) instead of underscores (_)"
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
        const NAME_ARG_FIRST: &[&str] = &[
            "g_signal_new",
            "g_signal_newv",
            "g_signal_new_valist",
            "g_signal_new_class_handler",
            "g_signal_lookup",
        ];
        const NAME_ARG_SECOND: &[&str] = &[
            "g_signal_connect",
            "g_signal_connect_after",
            "g_signal_connect_swapped",
            "g_signal_connect_data",
            "g_signal_connect_object",
            "g_signal_emit_by_name",
            "g_signal_group_connect",
            "g_signal_group_connect_after",
            "g_signal_group_connect_swapped",
            "g_signal_group_connect_object",
        ];

        for call in func.find_calls_matching(|name| {
            NAME_ARG_FIRST.contains(&name) || NAME_ARG_SECOND.contains(&name)
        }) {
            let name = call.function_name_str().unwrap();
            let arg_index = if NAME_ARG_FIRST.contains(&name) { 0 } else { 1 };
            if let Some(Argument::Expression(arg_expr)) = call.arguments.get(arg_index) {
                self.check_signal_name_arg(arg_expr, file, violations);
            }
        }
    }
}

impl SignalCanonicalName {
    /// Check a signal name argument (should be a string literal)
    fn check_signal_name_arg(
        &self,
        expr: &gobject_ast::Expression,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if let Expression::StringLiteral(string_lit) = expr {
            // Remove quotes and check for underscores
            let signal_name = string_lit.value.trim_matches('"');

            if signal_name.contains('_') {
                // Generate the fixed signal name (replace _ with -)
                let fixed_name = signal_name.replace('_', "-");
                let replacement = format!("\"{}\"", fixed_name);

                let fix = Fix::new(
                    string_lit.location.start_byte,
                    string_lit.location.end_byte,
                    replacement,
                );

                violations.push(self.violation_with_fix(
                    &file.path,
                    string_lit.location.line,
                    string_lit.location.column,
                    format!(
                        "Signal name '{}' should use hyphens instead of underscores: '{}'",
                        signal_name, fixed_name
                    ),
                    fix,
                ));
            }
        }
    }
}
