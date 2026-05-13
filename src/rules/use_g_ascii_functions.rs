use gobject_ast::model::{FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{FunctionRename, Rule, Violation},
};

const RENAMES: &[FunctionRename] = &[
    FunctionRename {
        from: "tolower",
        to: Some("g_ascii_tolower"),
        message: "Use g_ascii_tolower() instead of tolower() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "toupper",
        to: Some("g_ascii_toupper"),
        message: "Use g_ascii_toupper() instead of toupper() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isdigit",
        to: Some("g_ascii_isdigit"),
        message: "Use g_ascii_isdigit() instead of isdigit() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isalpha",
        to: Some("g_ascii_isalpha"),
        message: "Use g_ascii_isalpha() instead of isalpha() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isalnum",
        to: Some("g_ascii_isalnum"),
        message: "Use g_ascii_isalnum() instead of isalnum() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isspace",
        to: Some("g_ascii_isspace"),
        message: "Use g_ascii_isspace() instead of isspace() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isupper",
        to: Some("g_ascii_isupper"),
        message: "Use g_ascii_isupper() instead of isupper() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "islower",
        to: Some("g_ascii_islower"),
        message: "Use g_ascii_islower() instead of islower() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isxdigit",
        to: Some("g_ascii_isxdigit"),
        message: "Use g_ascii_isxdigit() instead of isxdigit() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "ispunct",
        to: Some("g_ascii_ispunct"),
        message: "Use g_ascii_ispunct() instead of ispunct() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isprint",
        to: Some("g_ascii_isprint"),
        message: "Use g_ascii_isprint() instead of isprint() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "isgraph",
        to: Some("g_ascii_isgraph"),
        message: "Use g_ascii_isgraph() instead of isgraph() — C ctype functions are locale-dependent",
    },
    FunctionRename {
        from: "iscntrl",
        to: Some("g_ascii_iscntrl"),
        message: "Use g_ascii_iscntrl() instead of iscntrl() — C ctype functions are locale-dependent",
    },
];

pub struct UseGAsciiFunctions;

impl Rule for UseGAsciiFunctions {
    fn name(&self) -> &'static str {
        "use_g_ascii_functions"
    }

    fn description(&self) -> &'static str {
        "Use g_ascii_* functions instead of locale-dependent C ctype functions"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_function_renames(func, file, config, violations, RENAMES);
    }
}
