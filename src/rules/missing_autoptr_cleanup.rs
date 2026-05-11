use std::collections::HashSet;

use gobject_ast::model::{
    DefineKind, GObjectTypeKind, PreprocessorDirective, SourceLocation, TopLevelItem,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct MissingAutoptrCleanup;

impl Rule for MissingAutoptrCleanup {
    fn name(&self) -> &'static str {
        "missing_autoptr_cleanup"
    }

    fn description(&self) -> &'static str {
        "Detect boxed types without G_DEFINE_AUTOPTR_CLEANUP_FUNC"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(
            "Detects types that don't have automatic g_autoptr() support:\n\
             - Boxed types (G_DEFINE_BOXED_TYPE*) without G_DEFINE_AUTOPTR_CLEANUP_FUNC\n\
             - Old-style GObject types (G_DEFINE_TYPE*) that should use G_DECLARE_* or have explicit cleanup\n\
             Modern GLib code should support g_autoptr() for automatic memory management.",
        )
    }

    fn category(&self) -> Category {
        Category::Style
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let mut types_needing_cleanup: Vec<(&std::path::Path, &str, SourceLocation, &'static str)> =
            Vec::new();
        let mut declared_types: HashSet<&str> = HashSet::new();
        let mut autoptr_cleanups: HashSet<&str> = HashSet::new();

        for (path, file) in ast_context.iter_all_files() {
            for gobject_type in file.iter_all_gobject_types() {
                match &gobject_type.kind {
                    GObjectTypeKind::DefineBoxed { .. }
                    | GObjectTypeKind::Define(DefineKind::Pointer) => {
                        types_needing_cleanup.push((
                            path,
                            &gobject_type.type_name,
                            gobject_type.location,
                            "boxed",
                        ));
                    }
                    GObjectTypeKind::Define(_) => {
                        types_needing_cleanup.push((
                            path,
                            &gobject_type.type_name,
                            gobject_type.location,
                            "old-style",
                        ));
                    }
                    GObjectTypeKind::Declare { .. } => {
                        declared_types.insert(&gobject_type.type_name);
                    }
                    GObjectTypeKind::DefineQuark { .. }
                    | GObjectTypeKind::DefineEnum { .. }
                    | GObjectTypeKind::DefineFlags { .. }
                    | GObjectTypeKind::DefineCustom { .. } => {}
                }
            }

            for item in file.iter_all_items() {
                if let TopLevelItem::Preprocessor(directive) = item
                    && let PreprocessorDirective::AutoptrCleanupFunc { type_name, .. } = directive
                {
                    autoptr_cleanups.insert(type_name);
                }
            }
        }

        for (path, type_name, location, kind) in types_needing_cleanup {
            if declared_types.contains(type_name) {
                continue;
            }

            if autoptr_cleanups.contains(type_name) {
                continue;
            }

            let message = match kind {
                "boxed" => format!(
                    "Boxed type '{}' is missing G_DEFINE_AUTOPTR_CLEANUP_FUNC macro",
                    type_name
                ),
                "old-style" => format!(
                    "GObject type '{}' defined with G_DEFINE_TYPE* should either use G_DECLARE_* or have G_DEFINE_AUTOPTR_CLEANUP_FUNC",
                    type_name
                ),
                _ => unreachable!(),
            };

            violations.push(self.violation(path, location.line, location.column, message));
        }
    }
}
