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
        let mut defined: HashSet<String> = HashSet::new();
        for (_, file) in ast_context.iter_all_files() {
            for f in file.iter_function_definitions() {
                defined.insert(f.name.clone());
            }
            for gt in file.iter_all_gobject_types() {
                defined.insert(format!("{}_get_type", gt.function_prefix));
            }
            // #define g_foo_get_type _g_foo_get_type
            for (name, value) in file.iter_defines() {
                if let Some(value) = value {
                    let raw = value.as_raw_str();
                    if raw.starts_with('_') && &raw[1..] == name {
                        defined.insert(raw.to_owned());
                    }
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
