use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct NoGAutoMacros;

impl Rule for NoGAutoMacros {
    fn name(&self) -> &'static str {
        "no_g_auto_macros"
    }

    fn description(&self) -> &'static str {
        "Forbid g_auto* macros (g_autoptr, g_autofree, etc.) for MSVC compatibility"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Portability
    }

    fn fixable(&self) -> bool {
        false
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Check all variable declarations in the function
        for stmt in &func.body_statements {
            for decl in stmt.iter_declarations() {
                if let Some(auto_macro) = &decl.type_info.auto_cleanup {
                    violations.push(self.violation(
                        &file.path,
                        decl.location.line,
                        decl.location.column,
                        format!("{auto_macro} requires compiler cleanup attribute support",),
                    ));
                }
            }
        }
    }
}
