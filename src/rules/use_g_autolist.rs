use std::collections::HashMap;

use gobject_ast::{Expression, expression::Argument, top_level::FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Rule, Violation},
};

pub struct UseGAutolist;

impl Rule for UseGAutolist {
    fn name(&self) -> &'static str {
        "use_g_autolist"
    }

    fn description(&self) -> &'static str {
        "Suggest g_autolist/g_autoslist instead of manual g_list_free_full/g_slist_free_full cleanup"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        self.check_function(func, file, violations);
    }
}

impl UseGAutolist {
    fn check_function(
        &self,
        func: &FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Find all GList*/GSList* declarations
        let list_vars: HashMap<&str, (&gobject_ast::TypeInfo, gobject_ast::SourceLocation)> = func
            .iter_local_declarations()
            .filter(|d| {
                !d.type_info.uses_auto_cleanup()
                    && (d.type_info.base_type == "GList" || d.type_info.base_type == "GSList")
                    && d.type_info.is_pointer()
                    && d.is_simple_identifier()
            })
            .map(|d| (d.name.as_str(), (&d.type_info, d.location)))
            .collect();

        // For each list variable, check if it's freed with
        // g_list_free_full/g_slist_free_full
        for (name, (type_info, location)) in &list_vars {
            let free_func = if type_info.base_type == "GList" {
                "g_list_free_full"
            } else {
                "g_slist_free_full"
            };

            if func.is_var_passed_to_function(name, free_func, 0) {
                // Skip if using basic free functions (g_free, free) as those indicate
                // primitive types (char*, etc.) that don't support g_autoptr
                if self.uses_basic_destructor(func, free_func) {
                    continue;
                }

                // Check if variable is returned (would need different handling)
                let is_returned = func.is_var_returned(type_info);

                if !is_returned {
                    let (auto_type, base_type) = match type_info.base_type.as_str() {
                        "GList" => ("g_autolist", "g_list"),
                        "GSList" => ("g_autoslist", "g_slist"),
                        _ => unreachable!(),
                    };

                    violations.push(self.violation(
                        &file.path,
                        location.line,
                        location.column,
                        format!(
                            "Consider using {auto_type} to avoid manual {base_type}_free_full cleanup",
                        ),
                    ));
                }
            }
        }
    }

    /// Check if any call to the free function uses a basic destructor (g_free,
    /// free, etc.) This indicates a list of primitive types that don't
    /// support g_autoptr
    fn uses_basic_destructor(&self, func: &FunctionDefItem, free_func: &str) -> bool {
        let calls = func.find_calls(&[free_func]);

        for call in calls {
            if call.arguments.len() >= 2 {
                let Argument::Expression(expr) = &call.arguments[1];
                if let Expression::Identifier(destructor) = expr.as_ref()
                    && matches!(
                        destructor.name.as_str(),
                        "g_free" | "free" | "g_slice_free" | "g_slice_free1"
                    )
                {
                    return true;
                }
            }
        }

        false
    }
}
