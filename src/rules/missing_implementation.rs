use std::collections::HashSet;

use crate::{ast_context::AstContext, config::Config, rules::Rule};

/// Rule that checks for functions declared in headers but never implemented
pub struct MissingImplementation;

impl Rule for MissingImplementation {
    fn name(&self) -> &'static str {
        "missing_implementation"
    }

    fn description(&self) -> &'static str {
        "Report functions declared in headers but not implemented"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Suspicious
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<crate::rules::Violation>,
    ) {
        let declared_types: HashSet<&str> = ast_context
            .iter_header_files()
            .flat_map(|(_, file)| file.iter_all_gobject_types())
            .filter(|gt| gt.kind.is_declare())
            .map(|gt| gt.type_name.as_str())
            .collect();

        // Collect explicit function definitions and _get_type() names from
        // G_DEFINE_* macros that lack a matching G_DECLARE_*.
        let mut defined: HashSet<String> = HashSet::new();
        for (_, file) in ast_context.iter_c_files() {
            for f in file.iter_function_definitions() {
                defined.insert(f.name.clone());
            }
            for gt in file.iter_all_gobject_types() {
                if gt.kind.is_define() && !declared_types.contains(gt.type_name.as_str()) {
                    defined.insert(format!("{}_get_type", gt.function_prefix));
                }
            }
        }

        for (path, file) in ast_context.iter_header_files() {
            for func in file.iter_function_declarations() {
                if func.is_static || func.name.ends_with("_quark") {
                    continue;
                }
                if defined.contains(func.name.as_str()) {
                    continue;
                }
                violations.push(self.violation(
                    path,
                    func.location.line,
                    1,
                    format!(
                        "Function '{}' is declared in a header but has no implementation",
                        func.name
                    ),
                ));
            }
        }
    }
}
