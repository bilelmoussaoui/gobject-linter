use gobject_ast::model::{Expression, FileModel, FunctionDefItem};

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
        func: &FunctionDefItem,
        file: &FileModel,
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
            "g_signal_stop_emission_by_name",
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
            if let Some(arg_expr) = call.arguments.get(arg_index) {
                self.check_signal_name_arg(arg_expr, file, violations);
            }
        }
    }
}

impl SignalCanonicalName {
    fn check_signal_name_arg(
        &self,
        expr: &Expression,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if let Expression::StringLiteral(string_lit) = expr {
            let raw = &string_lit.value;

            // Find the content of the first quoted portion.
            let Some(first_close) = raw[1..].find('"') else {
                return;
            };
            let first_str_content = &raw[1..1 + first_close];

            let signal_name = first_str_content
                .split("::")
                .next()
                .unwrap_or(first_str_content);

            if signal_name.contains('_') {
                let fixed_signal = signal_name.replace('_', "-");

                let fixed_first_str = format!(
                    "\"{}{}\"",
                    fixed_signal,
                    &first_str_content[signal_name.len()..],
                );

                let fix_start = string_lit.location.start_byte;
                let fix_end = fix_start + 1 + first_close + 1;

                let fix = Fix::new(fix_start, fix_end, fixed_first_str);

                violations.push(self.violation_with_fix_at(
                    &file.path,
                    &string_lit.location,
                    format!(
                        "Signal name '{}' should use hyphens instead of underscores: '{}'",
                        signal_name, fixed_signal
                    ),
                    fix,
                ));
            }
        }
    }
}
