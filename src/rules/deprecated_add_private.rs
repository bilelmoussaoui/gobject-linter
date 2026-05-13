use gobject_ast::model::{FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{FunctionRename, Rule, Violation},
};

const RENAMES: &[FunctionRename] = &[FunctionRename {
    from: "g_type_class_add_private",
    to: None,
    message: "g_type_class_add_private is deprecated since GLib 2.58. Use G_DEFINE_TYPE_WITH_PRIVATE or G_ADD_PRIVATE instead",
}];

pub struct DeprecatedAddPrivate;

impl Rule for DeprecatedAddPrivate {
    fn name(&self) -> &'static str {
        "deprecated_add_private"
    }

    fn description(&self) -> &'static str {
        "Detect deprecated g_type_class_add_private (use G_DEFINE_TYPE_WITH_PRIVATE instead)"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Restriction
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_function_renames(func, file, _config, violations, RENAMES);
    }
}
