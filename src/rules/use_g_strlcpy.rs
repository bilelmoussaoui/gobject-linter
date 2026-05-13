use gobject_ast::model::{FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{FunctionRename, Rule, Violation},
};

const RENAMES: &[FunctionRename] = &[
    FunctionRename {
        from: "strcpy",
        to: None,
        message: "Use g_strlcpy(dst, src, sizeof(dst)) instead of strcpy — no bounds checking",
    },
    FunctionRename {
        from: "strcat",
        to: None,
        message: "Use g_strlcat(dst, src, sizeof(dst)) instead of strcat — no bounds checking",
    },
    FunctionRename {
        from: "strncat",
        to: None,
        message: "Use g_strlcat(dst, src, sizeof(dst)) instead of strncat — strncat's n parameter is the max to append, not the buffer size, which is error-prone",
    },
];

pub struct UseGStrlcpy;

impl Rule for UseGStrlcpy {
    fn name(&self) -> &'static str {
        "use_g_strlcpy"
    }

    fn description(&self) -> &'static str {
        "Use g_strlcpy/g_strlcat instead of unsafe strcpy/strcat/strncat"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
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
