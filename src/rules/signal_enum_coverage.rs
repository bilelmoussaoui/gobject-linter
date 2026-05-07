use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct SignalEnumCoverage;

impl Rule for SignalEnumCoverage {
    fn name(&self) -> &'static str {
        "signal_enum_coverage"
    }

    fn description(&self) -> &'static str {
        "Ensure all signal enum values have corresponding g_signal_new calls"
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
            for enum_info in file.iter_all_enums() {
                if !enum_info.is_signal_enum() {
                    continue;
                }

                let signal_values: Vec<&str> = enum_info
                    .values
                    .iter()
                    .filter(|v| !v.is_signal_last())
                    .map(|v| v.name.as_str())
                    .collect();

                if signal_values.is_empty() {
                    continue;
                }

                let Some(gobject_type) = file.find_gobject_type_for_signal_enum(enum_info) else {
                    continue;
                };

                let installed: std::collections::HashSet<&str> = gobject_type
                    .signals
                    .iter()
                    .filter_map(|s| s.enum_value.as_deref())
                    .collect();

                for signal_name in &signal_values {
                    if !installed.contains(signal_name) {
                        violations.push(self.violation(
                            path,
                            enum_info.location.line,
                            1,
                            format!(
                                "Signal enum value '{}' is declared but never installed in {}",
                                signal_name,
                                gobject_type.class_init_function_name()
                            ),
                        ));
                    }
                }
            }
        }
    }
}
