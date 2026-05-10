use gobject_ast::model::{FileModel, FunctionDeclItem, GObjectType};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct MissingExportMacro;

impl Rule for MissingExportMacro {
    fn name(&self) -> &'static str {
        "missing_export_macro"
    }

    fn description(&self) -> &'static str {
        "Detect public API functions and types without export macros"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(
            "Detects functions and types in public headers that lack export macros.\n\
             Public API should be marked with appropriate export macros (e.g., G_MODULE_EXPORT, \
             CLUTTER_EXPORT, META_EXPORT) to ensure proper symbol visibility.\n\
             Public API should be marked with appropriate export macros (e.g., G_MODULE_EXPORT, \
             CLUTTER_EXPORT, META_EXPORT) to ensure proper symbol visibility.",
        )
    }

    fn category(&self) -> Category {
        Category::Correctness
    }

    fn requires_meson(&self) -> bool {
        true
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn opt_in_reason(&self) -> Option<&'static str> {
        Some(
            "May produce false positives as the parser can mis-identify export macros in some codebases",
        )
    }

    fn check_func_decl(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDeclItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if !ast_context.is_public_header(&file.path).unwrap_or(false) {
            return;
        }

        if func.is_static {
            return;
        }

        if func.export_macros.is_empty() {
            violations.push(self.violation(
                &file.path,
                func.location.line,
                func.location.column,
                format!(
                    "Public function '{}' in header is missing an export macro (e.g., G_MODULE_EXPORT, *_EXPORT)",
                    func.name
                ),
            ));
        }
    }

    fn check_gobject_type(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        gobject_type: &GObjectType,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if !ast_context.is_public_header(&file.path).unwrap_or(false) {
            return;
        }

        if !gobject_type.kind.is_declare() {
            return;
        }

        if gobject_type.export_macros.is_empty() {
            violations.push(self.violation(
                &file.path,
                gobject_type.location.line,
                gobject_type.location.column,
                format!(
                    "'{}' is missing an export macro (e.g., G_MODULE_EXPORT, *_EXPORT)",
                    gobject_type.type_name
                ),
            ));
        }
    }
}
