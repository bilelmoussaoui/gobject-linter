use gobject_ast::model::{CallExpression, FileModel, FunctionDefItem};

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
        for call in func.find_calls(&["g_string_free"]) {
            self.check_call(file, call, config, violations);
        }
    }
}

impl UseGStringFreeAndSteal {
    fn check_call(
        &self,
        file: &FileModel,
        call: &CallExpression,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() != 2 {
            return;
        }

        // Check if second argument is FALSE/false/0
        let Some(second_expr) = call.get_arg(1) else {
            return;
        };

        if !second_expr.is_falsy() {
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
        let replacement = config
            .style
            .format_call("g_string_free_and_steal", &[first_text]);
        let message = format!(
            "Use {} instead of g_string_free({}, {}) for readability",
            replacement, first_text, second_text
        );
        let fix = Fix::new(
            call.location.start_byte,
            call.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix_at(&file.path, &call.location, message, fix));
    }
}
