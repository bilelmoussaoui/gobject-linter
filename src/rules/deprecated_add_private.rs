use gobject_ast::model::{FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

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
        for call in func.find_calls(&["g_type_class_add_private"]) {
            violations.push(self.violation_at(
                &file.path,
                &call.location,
                "g_type_class_add_private is deprecated since GLib 2.58. Use G_DEFINE_TYPE_WITH_PRIVATE or G_ADD_PRIVATE instead".to_string(),
            ));
        }
    }
}
