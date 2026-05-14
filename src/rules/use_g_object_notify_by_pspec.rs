use std::collections::HashMap;

use gobject_ast::model::{
    CallExpression, Expression, FileModel, FunctionDefItem, ParamSpecAssignment, Parameter,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGObjectNotifyByPspec;

struct PropertyEntry<'a> {
    assignment: &'a ParamSpecAssignment,
    class_prefix: &'a str,
}

impl Rule for UseGObjectNotifyByPspec {
    fn name(&self) -> &'static str {
        "use_g_object_notify_by_pspec"
    }

    fn description(&self) -> &'static str {
        "Suggest g_object_notify_by_pspec instead of g_object_notify for better performance"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Perf
    }

    fn fixable(&self) -> bool {
        true
    }

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 26))
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_all_files() {
            let source = &file.source;
            let property_map = self.build_property_map(file);

            for func in file.iter_function_definitions() {
                for call in func.find_calls(&["g_object_notify"]) {
                    self.check_call(
                        path,
                        call,
                        source,
                        &property_map,
                        func,
                        &config.style,
                        violations,
                    );
                }
            }
        }
    }
}

impl UseGObjectNotifyByPspec {
    #[allow(clippy::too_many_arguments)]
    fn check_call(
        &self,
        file_path: &std::path::Path,
        call: &CallExpression,
        source: &[u8],
        property_map: &HashMap<&str, Vec<PropertyEntry>>,
        func: &FunctionDefItem,
        style: &crate::config::Style,
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() != 2 {
            return;
        }

        let Some(property_expr) = call.get_arg(1) else {
            return;
        };
        let Expression::StringLiteral(string_lit) = property_expr else {
            return;
        };

        let property_name = string_lit.value.trim_matches('"');

        let Some(candidates) = property_map.get(property_name) else {
            // Property not found in any GObject type
            let property_constant = self.property_name_to_constant(property_name);
            violations.push(self.violation_at(
                file_path,
                &call.location,
                format!(
                    "Use g_object_notify_by_pspec(obj, properties[{}]) instead of g_object_notify(obj, \"{}\") for better performance",
                    property_constant, property_name
                ),
            ));
            return;
        };

        // Filter to only array-subscript candidates (the only ones we can fix)
        let fixable: Vec<_> = candidates
            .iter()
            .filter(|e| matches!(e.assignment, ParamSpecAssignment::ArraySubscript { .. }))
            .collect();

        if fixable.is_empty() {
            // Property exists but only as override/direct-install — can't use by_pspec
            return;
        }

        let disambiguated = if fixable.len() > 1 {
            self.disambiguate_by_type(call, func, &fixable)
        } else {
            Some(fixable[0])
        };

        if let Some(entry) = disambiguated {
            let ParamSpecAssignment::ArraySubscript {
                array_name,
                enum_value,
                ..
            } = entry.assignment
            else {
                return;
            };

            let Some(obj_expr) = call.get_arg(0) else {
                return;
            };
            let Some(obj_str) = obj_expr.to_source_string(source) else {
                return;
            };

            let pspec = format!("{}[{}]", array_name, enum_value);
            let replacement = style.format_call("g_object_notify_by_pspec", &[obj_str, &pspec]);

            violations.push(self.violation_with_fix_at(
                file_path,
                &call.location,
                format!(
                    "Use g_object_notify_by_pspec({}, {}[{}]) instead of g_object_notify({}, \"{}\") for better performance",
                    obj_str, array_name, enum_value, obj_str, property_name
                ),
                Fix::new(call.location.start_byte, call.location.end_byte, replacement),
            ));
        } else {
            let property_constant = self.property_name_to_constant(property_name);
            let ParamSpecAssignment::ArraySubscript { array_name, .. } = fixable[0].assignment
            else {
                return;
            };
            violations.push(self.violation_at(
                file_path,
                &call.location,
                format!(
                    "Use g_object_notify_by_pspec(obj, {}[{}]) instead of g_object_notify(obj, \"{}\") for better performance (ambiguous: multiple classes define this property)",
                    array_name, property_constant, property_name
                ),
            ));
        }
    }

    fn build_property_map<'a>(
        &self,
        file: &'a FileModel,
    ) -> HashMap<&'a str, Vec<PropertyEntry<'a>>> {
        let mut map: HashMap<&str, Vec<PropertyEntry>> = HashMap::new();

        for gt in file.iter_all_gobject_types() {
            for assignment in &gt.properties {
                map.entry(assignment.property().name.as_str())
                    .or_default()
                    .push(PropertyEntry {
                        assignment,
                        class_prefix: &gt.function_prefix,
                    });
            }
        }

        map
    }

    fn disambiguate_by_type<'a>(
        &self,
        call: &CallExpression,
        func: &FunctionDefItem,
        candidates: &[&'a PropertyEntry<'a>],
    ) -> Option<&'a PropertyEntry<'a>> {
        let obj_expr = call.get_arg(0)?;
        let obj_identifier = obj_expr.extract_identifier_name()?;

        let param_type = func.get_param_by_name(obj_identifier).and_then(|p| {
            if let Parameter::Regular { type_info, .. } = p {
                Some(&type_info.base_type)
            } else {
                None
            }
        })?;

        use heck::ToSnakeCase;
        let full = param_type.to_snake_case();
        if let Some(found) = candidates.iter().find(|e| e.class_prefix == full) {
            return Some(found);
        }
        let trimmed = param_type
            .trim_end_matches("Object")
            .trim_end_matches("Class")
            .to_snake_case();
        candidates
            .iter()
            .find(|e| e.class_prefix == trimmed)
            .copied()
    }

    /// Convert property-name to PROP_NAME constant style
    fn property_name_to_constant(&self, property_name: &str) -> String {
        let mut result = String::with_capacity(property_name.len() + 5);
        result.push_str("PROP_");

        for c in property_name.chars() {
            if c == '-' {
                result.push('_');
            } else {
                result.push(c.to_ascii_uppercase());
            }
        }

        result
    }
}
