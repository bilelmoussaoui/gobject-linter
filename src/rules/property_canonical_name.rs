use gobject_ast::model::{CallExpression, Expression, FileModel, GObjectType, ParamFlag, Property};

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
            self.check_call(file, call, assignment.property(), violations);
        }
    }
}

impl PropertyCanonicalName {
    fn check_call(
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
}
