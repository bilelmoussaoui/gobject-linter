use gobject_ast::model::{FileModel, PreprocessorDirective, SourceLocation, TopLevelItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseTgmath;

/// Type-generic macros defined by C99 <tgmath.h>
const TG_MATH_FUNCTIONS: &[&str] = &[
    "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh", "cbrt", "cos", "cosh", "erf",
    "erfc", "exp", "fabs", "fmod", "hypot", "ldexp", "lgamma", "log", "pow", "sin", "sinh", "sqrt",
    "tan", "tanh", "tgamma",
];

/// Map a typed variant like sinf/sinl to its type-generic macro sin
fn generic_math_fn(name: &str) -> Option<&'static str> {
    let base = name.strip_suffix('f').or_else(|| name.strip_suffix('l'))?;
    TG_MATH_FUNCTIONS.iter().find(|f| **f == base).copied()
}

/// Collect include directives, descending into preprocessor conditionals
fn collect_includes<'a>(items: &'a [TopLevelItem], out: &mut Vec<(&'a str, &'a SourceLocation)>) {
    for item in items {
        match item {
            TopLevelItem::Preprocessor(PreprocessorDirective::Include {
                path, location, ..
            }) => {
                out.push((path, location));
            }
            TopLevelItem::Preprocessor(PreprocessorDirective::Conditional { body, .. }) => {
                collect_includes(body, out);
            }
            _ => {}
        }
    }
}

impl Rule for UseTgmath {
    fn name(&self) -> &'static str {
        "use_tgmath"
    }

    fn description(&self) -> &'static str {
        "Use <tgmath.h> type-generic math functions instead of explicit variants like sinf()"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/use_tgmath.md"))
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (_path, file) in ast_context.iter_all_files() {
            self.check_file(file, violations);
        }
    }
}

impl UseTgmath {
    fn check_file(&self, file: &FileModel, violations: &mut Vec<Violation>) {
        let mut includes = Vec::new();
        collect_includes(&file.top_level_items, &mut includes);

        let mut math_include = None;
        for (path, location) in includes {
            match path {
                // <tgmath.h> already provides the type-generic macros
                "tgmath.h" => return,
                "math.h" => math_include = math_include.or(Some(location)),
                _ => {}
            }
        }
        let Some(math_include) = math_include else {
            return;
        };

        let mut include_fix_emitted = false;
        for func in file.iter_function_definitions() {
            for call in func.find_calls_matching(|name| generic_math_fn(name).is_some()) {
                let Some(name) = call.function_name_str() else {
                    continue;
                };
                let Some(generic) = generic_math_fn(name) else {
                    continue;
                };

                let message =
                    format!("Use {generic}() from <tgmath.h> instead of {name}() from <math.h>");
                let fn_location = call.function.location();
                let mut fixes = vec![Fix::new(
                    fn_location.start_byte,
                    fn_location.end_byte,
                    generic,
                )];
                // Replace the <math.h> include once, with the first violation
                if !include_fix_emitted && let Some(text) = math_include.as_str() {
                    include_fix_emitted = true;
                    fixes.push(Fix::new(
                        math_include.start_byte,
                        math_include.end_byte,
                        text.replace("math.h", "tgmath.h"),
                    ));
                }

                violations.push(self.violation_with_fixes_at(
                    &file.path,
                    &call.location,
                    message,
                    fixes,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_typed_variants() {
        assert_eq!(generic_math_fn("sinf"), Some("sin"));
        assert_eq!(generic_math_fn("sinl"), Some("sin"));
        assert_eq!(generic_math_fn("powl"), Some("pow"));
        assert_eq!(generic_math_fn("fabsf"), Some("fabs"));
        assert_eq!(generic_math_fn("ldexpl"), Some("ldexp"));
    }

    #[test]
    fn rejects_non_generic_names() {
        // Generic macros themselves
        assert_eq!(generic_math_fn("sin"), None);
        assert_eq!(generic_math_fn("fabs"), None);
        // Not type-generic in <tgmath.h>
        assert_eq!(generic_math_fn("log2f"), None);
        assert_eq!(generic_math_fn("roundf"), None);
        assert_eq!(generic_math_fn("modff"), None);
        assert_eq!(generic_math_fn("ceilf"), None);
    }
}
