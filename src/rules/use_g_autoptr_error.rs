use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct UseGAutoptrError;

impl Rule for UseGAutoptrError {
    fn name(&self) -> &'static str {
        "use_g_autoptr_error"
    }

    fn description(&self) -> &'static str {
        "Suggest g_autoptr(GError) instead of manual g_error_free"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_function(func, file, violations);
    }
}

impl UseGAutoptrError {
    fn check_function(
        &self,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Find all GError* declarations
        let gerror_vars: Vec<(&str, (&gobject_ast::TypeInfo, gobject_ast::SourceLocation))> = func
            .iter_local_declarations()
            .filter(|d| {
                !d.type_info.uses_auto_cleanup()
                    && d.type_info.is_base_type("GError")
                    && d.type_info.is_pointer()
            })
            .map(|d| (d.name.as_str(), (&d.type_info, d.location)))
            .collect();

        // For each GError* variable, check if it's manually freed
        for (var_name, (_type_info, location)) in &gerror_vars {
            if func.is_var_passed_to_function(var_name, "g_error_free", 0) {
                violations.push(self.violation(
                    &file.path,
                    location.line,
                    location.column,
                    format!(
                        "Consider using g_autoptr(GError) {} instead of manual g_error_free",
                        var_name
                    ),
                ));
            }
        }
    }
}
