use gobject_ast::model::{FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{FunctionRename, Rule, Violation},
};

const RENAMES: &[FunctionRename] = &[FunctionRename {
    from: "strcmp",
    to: Some("g_strcmp0"),
    message: "Use g_strcmp0() instead of strcmp() — g_strcmp0 is NULL-safe",
}];

pub struct UseGStrcmp0;

impl Rule for UseGStrcmp0 {
    fn name(&self) -> &'static str {
        "use_g_strcmp0"
    }

    fn description(&self) -> &'static str {
        "Suggest g_strcmp0 instead of strcmp if arguments can be NULL (NULL-safe)"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 16))
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
