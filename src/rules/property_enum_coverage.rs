use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct PropertyEnumCoverage;

impl Rule for PropertyEnumCoverage {
    fn name(&self) -> &'static str {
        "property_enum_coverage"
    }

    fn description(&self) -> &'static str {
        "Ensure all property enum values have corresponding g_param_spec or g_object_class_override_property"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_all_files() {
            for enum_info in file.iter_property_enums() {
                // Get all property enum values (excluding PROP_0 and N_PROPS)
                let property_values: Vec<&str> = enum_info
                    .values
                    .iter()
                    .filter(|v| !v.is_prop_0() && !v.is_prop_last())
                    .map(|v| v.name.as_str())
                    .collect();

                if property_values.is_empty() {
                    continue;
                }

                // Find which GObjectType corresponds to this enum
                let Some(gobject_type) = file.find_gobject_type_for_property_enum(enum_info) else {
                    continue;
                };

                // Collect all installed property enum values
                let installed_properties: Vec<_> = gobject_type
                    .properties
                    .iter()
                    .filter_map(|assignment| assignment.get_installed_enum_value(&file.source))
                    .collect();

                // Check coverage
                for prop_name in property_values {
                    if !installed_properties.iter().any(|p| p == prop_name) {
                        // Find the enum value location for better error reporting
                        // We'll use the enum's location since we don't have per-value line/column
                        violations.push(self.violation(
                            path,
                            enum_info.location.line,
                            1,
                            format!(
                                "Property enum value '{}' is declared but never installed in {}",
                                prop_name,
                                gobject_type.class_init_function_name()
                            ),
                        ));
                    }
                }
            }
        }
    }
}
