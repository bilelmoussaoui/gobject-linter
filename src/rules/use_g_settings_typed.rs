use gobject_ast::{CallExpression, Expression};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGSettingsTyped;

impl Rule for UseGSettingsTyped {
    fn name(&self) -> &'static str {
        "use_g_settings_typed"
    }

    fn description(&self) -> &'static str {
        "Prefer g_settings_get/set_string/boolean/etc over g_settings_get/set_value with g_variant"
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
        config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Check for g_settings_set_value calls
        for call in func.find_calls(&["g_settings_set_value"]) {
            self.check_settings_set_call(config, file, call, violations);
        }

        // Check for g_variant_get_* calls
        for call in func.find_calls(&[
            "g_variant_get_string",
            "g_variant_get_boolean",
            "g_variant_get_byte",
            "g_variant_get_int16",
            "g_variant_get_uint16",
            "g_variant_get_int32",
            "g_variant_get_uint32",
            "g_variant_get_int64",
            "g_variant_get_uint64",
            "g_variant_get_double",
            "g_variant_get_strv",
        ]) {
            self.check_variant_get_call(config, file, call, violations);
        }
    }
}

impl UseGSettingsTyped {
    fn check_settings_set_call(
        &self,
        config: &Config,
        file: &gobject_ast::FileModel,
        call: &CallExpression,
        violations: &mut Vec<Violation>,
    ) {
        // g_settings_set_value(settings, key, variant)
        if call.arguments.len() != 3 {
            return;
        }

        // Check if third argument is g_variant_new call
        let Some(third_expr) = call.get_arg(2) else {
            return;
        };
        let Expression::Call(variant_call) = third_expr else {
            return;
        };

        if !variant_call.is_function("g_variant_new") {
            return;
        }

        // Extract the pattern from g_variant_new
        let Some((_format_str, typed_func, value_args)) =
            self.extract_variant_pattern(variant_call, file)
        else {
            return;
        };

        let Some(settings_arg) = call.get_arg_text(0, &file.source) else {
            return;
        };
        let Some(key_arg) = call.get_arg_text(1, &file.source) else {
            return;
        };

        // Build replacement
        let mut args: Vec<&str> = vec![settings_arg, key_arg];
        if !value_args.is_empty() {
            args.push(&value_args);
        }
        let replacement = config.format_call(typed_func, &args);
        let message = format!(
            "Use {} instead of g_settings_set_value with g_variant_new for type safety",
            replacement
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

    fn check_variant_get_call(
        &self,
        config: &Config,
        file: &gobject_ast::FileModel,
        call: &CallExpression,
        violations: &mut Vec<Violation>,
    ) {
        // g_variant_get_*(variant, ...) - first arg should be g_settings_get_value call
        if call.arguments.is_empty() {
            return;
        }

        // Check if first argument is g_settings_get_value call
        let Some(first_expr) = call.get_arg(0) else {
            return;
        };
        let Expression::Call(inner_call) = first_expr else {
            return;
        };

        if !inner_call.is_function("g_settings_get_value") {
            return;
        }

        // g_settings_get_value(settings, key)
        if inner_call.arguments.len() < 2 {
            return;
        }

        let Some(settings_arg) = inner_call.get_arg_text(0, &file.source) else {
            return;
        };
        let Some(key_arg) = inner_call.get_arg_text(1, &file.source) else {
            return;
        };

        // Map g_variant_get_* to g_settings_get_*
        let Some(func_name) = call.function_name_str() else {
            return;
        };
        let typed_func = match func_name {
            "g_variant_get_string" => "g_settings_get_string",
            "g_variant_get_boolean" => "g_settings_get_boolean",
            "g_variant_get_byte" => "g_settings_get_byte",
            "g_variant_get_int16" => "g_settings_get_int",
            "g_variant_get_uint16" => "g_settings_get_uint",
            "g_variant_get_int32" => "g_settings_get_int",
            "g_variant_get_uint32" => "g_settings_get_uint",
            "g_variant_get_int64" => "g_settings_get_int64",
            "g_variant_get_uint64" => "g_settings_get_uint64",
            "g_variant_get_double" => "g_settings_get_double",
            "g_variant_get_strv" => "g_settings_get_strv",
            _ => return,
        };

        // Build replacement
        let replacement = config.format_call(typed_func, &[settings_arg, key_arg]);
        let message = format!(
            "Use {} instead of g_variant_get_* with g_settings_get_value for type safety",
            replacement
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

    /// Extract g_variant_new pattern and return (format_string,
    /// typed_function_name, rest_of_args)
    fn extract_variant_pattern(
        &self,
        variant_call: &CallExpression,
        file: &gobject_ast::FileModel,
    ) -> Option<(String, &'static str, String)> {
        // Need at least 1 argument (the format string)
        if variant_call.arguments.is_empty() {
            return None;
        }

        // Check if first argument is a string literal
        let first_expr = variant_call.get_arg(0)?;
        let Expression::StringLiteral(string_lit) = first_expr else {
            return None;
        };

        let format_str = string_lit.value.trim_matches('"');

        // Map format string to typed settings function
        let typed_func = match format_str {
            "s" => "g_settings_set_string",
            "b" => "g_settings_set_boolean",
            "y" => "g_settings_set_byte",
            "n" => "g_settings_set_int",  // int16 → int (closest match)
            "q" => "g_settings_set_uint", // uint16 → uint (closest match)
            "i" => "g_settings_set_int",
            "u" => "g_settings_set_uint",
            "x" => "g_settings_set_int64",
            "t" => "g_settings_set_uint64",
            "d" => "g_settings_set_double",
            "as" => "g_settings_set_strv",
            _ => return None, // Not a simple type we can convert
        };

        // Collect remaining arguments (after format string)
        let rest_args = if variant_call.arguments.len() > 1 {
            let rest: Vec<&str> = variant_call.arguments[1..]
                .iter()
                .filter_map(|arg| arg.to_source_string(&file.source))
                .collect();
            rest.join(", ")
        } else {
            String::new()
        };

        Some((format_str.to_string(), typed_func, rest_args))
    }
}
