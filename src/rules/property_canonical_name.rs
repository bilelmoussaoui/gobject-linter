use gobject_ast::model::{
    CallExpression, Expression, FileModel, FunctionDefItem, GObjectType, ParamFlag, Property,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct PropertyCanonicalName;

impl Rule for PropertyCanonicalName {
    fn name(&self) -> &'static str {
        "property_canonical_name"
    }

    fn description(&self) -> &'static str {
        "Ensure property names are canonical (use dashes, not underscores)"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_gobject_type(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        gobject_type: &GObjectType,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for assignment in &gobject_type.properties {
            let Some(call) = assignment.param_spec_call() else {
                continue;
            };
            self.check_param_spec_call(file, call, assignment.property(), violations);
        }
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        const NAME_ARG_SECOND: &[&str] = &[
            "g_object_notify",
            "g_object_set_property",
            "g_object_get_property",
            "g_object_class_find_property",
        ];
        const NAME_ARG_THIRD: &[&str] = &["g_object_class_override_property"];
        const VARARGS_PROP_VALUE: &[&str] = &[
            "g_object_set",
            "g_object_get",
            "g_object_new",
            "g_object_new_with_properties",
        ];

        for call in func.find_calls_matching(|name| {
            NAME_ARG_SECOND.contains(&name)
                || NAME_ARG_THIRD.contains(&name)
                || VARARGS_PROP_VALUE.contains(&name)
        }) {
            let name = call.function_name_str().unwrap();

            if NAME_ARG_SECOND.contains(&name) {
                if let Some(arg) = call.arguments.get(1) {
                    self.check_property_name_arg(arg, file, violations);
                }
            } else if NAME_ARG_THIRD.contains(&name) {
                if let Some(arg) = call.arguments.get(2) {
                    self.check_property_name_arg(arg, file, violations);
                }
            } else {
                // g_object_set/get/new: "prop", value, "prop", value, ..., NULL
                // Property names at odd indices starting from 1 (for set/get)
                // or 1 (for g_object_new where arg 0 is the type)
                for arg in call.arguments.iter().skip(1).step_by(2) {
                    self.check_property_name_arg(arg, file, violations);
                }
            }
        }
    }
}

impl PropertyCanonicalName {
    fn check_param_spec_call(
        &self,
        file: &FileModel,
        call: &CallExpression,
        property: &Property,
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() < 2 {
            return;
        }

        if !property.name.contains('_') {
            return;
        }

        let has_static_name = property.flags.contains(&ParamFlag::StaticName)
            || property.flags.contains(&ParamFlag::StaticStrings);

        let name_value = &property.name;
        let canonical_name = name_value.replace('_', "-");
        let replacement = format!("\"{}\"", canonical_name);

        let Some(expr) = call.get_arg(0) else {
            return;
        };

        let string_lit_location = match expr {
            Expression::StringLiteral(lit) => &lit.location,
            _ => return,
        };

        let fix = Fix::new(
            string_lit_location.start_byte,
            string_lit_location.end_byte,
            replacement,
        );

        let message = if has_static_name {
            format!(
                "Property name '{}' is not canonical (contains underscores). \
                     With G_PARAM_STATIC_NAME this will cause: \
                     g_param_spec_internal: assertion '!(flags & G_PARAM_STATIC_NAME) || is_canonical (name)' failed. \
                     Use '{}' instead",
                name_value, canonical_name
            )
        } else {
            format!(
                "Property name '{}' should use dashes instead of underscores. \
                     Use '{}' for consistency with GObject conventions",
                name_value, canonical_name
            )
        };

        violations.push(self.violation_with_fix_at(&file.path, string_lit_location, message, fix));
    }

    fn check_property_name_arg(
        &self,
        expr: &Expression,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let Expression::StringLiteral(string_lit) = expr else {
            return;
        };
        let raw = &string_lit.value;

        let Some(first_close) = raw[1..].find('"') else {
            return;
        };
        let prop_name = &raw[1..1 + first_close];

        if !prop_name.contains('_') {
            return;
        }

        let canonical = prop_name.replace('_', "-");
        let replacement = format!("\"{}\"", canonical);

        let fix = Fix::new(
            string_lit.location.start_byte,
            string_lit.location.start_byte + 1 + first_close + 1,
            replacement,
        );

        violations.push(self.violation_with_fix_at(
            &file.path,
            &string_lit.location,
            format!(
                "Property name '{}' should use dashes instead of underscores: '{}'",
                prop_name, canonical
            ),
            fix,
        ));
    }
}
