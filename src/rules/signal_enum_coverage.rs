use gobject_ast::model::{EnumInfo, FileModel};

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

    fn check_enum(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        enum_info: &EnumInfo,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if !enum_info.is_signal_enum() {
            return;
        }

        let signal_values: Vec<&str> = enum_info
            .values
            .iter()
            .filter(|v| !v.is_signal_last())
            .map(|v| v.name.as_str())
            .collect();

        if signal_values.is_empty() {
            return;
        }

        let Some(gobject_type) = file.find_gobject_type_for_signal_enum(enum_info) else {
            return;
        };

        let installed: std::collections::HashSet<&str> = gobject_type
            .signals
            .iter()
            .filter_map(|s| s.enum_value.as_deref())
            .collect();

        for signal_name in &signal_values {
            if !installed.contains(signal_name) {
                violations.push(self.violation(
                    &file.path,
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
