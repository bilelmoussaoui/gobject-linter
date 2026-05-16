use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use gobject_ast::model::{
    DefineKind, GObjectTypeKind, PreprocessorDirective, SourceLocation, TopLevelItem,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Fix, Rule, Violation},
};

pub struct MissingAutoptrCleanup;

enum CleanupKind<'a> {
    Boxed { free_func: &'a str },
    OldStyleGObject,
    Pointer,
}

impl CleanupKind<'_> {
    fn cleanup_func(&self) -> Option<&str> {
        match self {
            CleanupKind::Boxed { free_func } => Some(free_func),
            CleanupKind::OldStyleGObject => Some("g_object_unref"),
            CleanupKind::Pointer => None,
        }
    }
}

struct GetTypeDecl<'a> {
    path: &'a Path,
    location: &'a SourceLocation,
    decls_block: Option<&'a SourceLocation>,
}

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

    fn fixable(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let mut types_needing_cleanup: Vec<(&Path, &str, &str, &SourceLocation, CleanupKind)> =
            Vec::new();
        let mut declared_types: HashSet<&str> = HashSet::new();
        let mut autoptr_cleanups: HashSet<&str> = HashSet::new();

        // Collect function declarations from headers
        let mut get_type_decls: HashMap<&str, GetTypeDecl> = HashMap::new();
        let mut header_func_names: HashSet<&str> = HashSet::new();
        for (path, file) in ast_context.iter_header_files() {
            // Find the GObjectDeclsBlock (G_BEGIN_DECLS/G_END_DECLS) in this header
            let decls_block = file.iter_all_items().find_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock {
                    location,
                    ..
                }) => Some(location),
                _ => None,
            });

            for decl in file.iter_function_declarations() {
                header_func_names.insert(&decl.name);
                if let Some(prefix) = decl.name.strip_suffix("_get_type") {
                    get_type_decls.insert(
                        prefix,
                        GetTypeDecl {
                            path,
                            location: &decl.location,
                            decls_block,
                        },
                    );
                }
            }
        }

        for (path, file) in ast_context.iter_all_files() {
            for gobject_type in file.iter_all_gobject_types() {
                match &gobject_type.kind {
                    GObjectTypeKind::DefineBoxed { free_func, .. } => {
                        types_needing_cleanup.push((
                            path,
                            &gobject_type.type_name,
                            &gobject_type.function_prefix,
                            &gobject_type.location,
                            CleanupKind::Boxed { free_func },
                        ));
                    }
                    GObjectTypeKind::Define(DefineKind::Pointer) => {
                        types_needing_cleanup.push((
                            path,
                            &gobject_type.type_name,
                            &gobject_type.function_prefix,
                            &gobject_type.location,
                            CleanupKind::Pointer,
                        ));
                    }
                    GObjectTypeKind::Define(_) => {
                        types_needing_cleanup.push((
                            path,
                            &gobject_type.type_name,
                            &gobject_type.function_prefix,
                            &gobject_type.location,
                            CleanupKind::OldStyleGObject,
                        ));
                    }
                    GObjectTypeKind::Declare { .. } => {
                        if !gobject_type.manually_registered {
                            declared_types.insert(&gobject_type.type_name);
                        }
                    }
                    _ => {}
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

        for (define_path, type_name, func_prefix, define_location, kind) in &types_needing_cleanup {
            if declared_types.contains(type_name) {
                continue;
            }

            if autoptr_cleanups.contains(type_name) {
                continue;
            }

            if let CleanupKind::Boxed { free_func } = kind {
                if !header_func_names.contains(*free_func) {
                    continue;
                }
            }

            let cleanup_func = kind.cleanup_func();

            if let Some(decl) = get_type_decls.get(*func_prefix) {
                let message = Self::make_message(type_name, kind);
                let fix = cleanup_func.map(|func| Self::make_fix(decl, type_name, func));

                let violation = if let Some(fix) = fix {
                    self.violation_with_fix_at(decl.path, decl.location, message, fix)
                } else {
                    self.violation_at(decl.path, decl.location, message)
                };
                violations.push(violation);
            } else {
                let message = Self::make_message(type_name, kind);
                violations.push(self.violation_at(define_path, define_location, message));
            }
        }
    }
}

impl MissingAutoptrCleanup {
    fn make_message(type_name: &str, kind: &CleanupKind) -> String {
        match kind {
            CleanupKind::Boxed { .. } => {
                format!(
                    "Boxed type '{}' is missing G_DEFINE_AUTOPTR_CLEANUP_FUNC macro",
                    type_name
                )
            }
            CleanupKind::OldStyleGObject => {
                format!(
                    "GObject type '{}' defined with G_DEFINE_TYPE* should either use G_DECLARE_* or have G_DEFINE_AUTOPTR_CLEANUP_FUNC",
                    type_name
                )
            }
            CleanupKind::Pointer => {
                format!(
                    "Pointer type '{}' is missing G_DEFINE_AUTOPTR_CLEANUP_FUNC macro",
                    type_name
                )
            }
        }
    }

    fn make_fix(decl: &GetTypeDecl, type_name: &str, cleanup_func: &str) -> Fix {
        let macro_text = format!("G_DEFINE_AUTOPTR_CLEANUP_FUNC ({type_name}, {cleanup_func})\n");

        if let Some(block_loc) = decl.decls_block {
            // Insert before G_END_DECLS
            let source = block_loc.source();
            let mut pos = block_loc.end_byte;
            while pos > 0 && source[pos - 1] != b'\n' {
                pos -= 1;
            }
            let has_blank = pos >= 2 && source[pos - 1] == b'\n' && source[pos - 2] == b'\n';
            let prefix = if has_blank { "" } else { "\n" };
            Fix::new(pos, pos, format!("{prefix}{macro_text}"))
        } else {
            let insert_pos = decl.location.find_line_range().0;
            Fix::new(insert_pos, insert_pos, format!("{macro_text}\n"))
        }
    }
}
