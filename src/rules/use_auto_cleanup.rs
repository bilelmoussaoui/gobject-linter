use std::{collections::HashMap, sync::LazyLock};

use globset::{Glob, GlobSet, GlobSetBuilder};
use gobject_ast::model::{
    Argument, Expression, FileModel, FunctionDefItem, SourceLocation, Statement, TypeInfo,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{ConfigOption, Rule, Violation},
};

const AUTOFREE_ALLOCATIONS: &[&str] = &[
    "g_strdup",
    "g_strndup",
    "g_strdup_printf",
    "g_strdup_vprintf",
    "g_malloc",
    "g_malloc0",
    "g_realloc",
    "g_try_malloc",
    "g_try_malloc0",
    "g_memdup",
    "g_new",
    "g_new0",
];

pub struct UseAutoCleanup;

impl Rule for UseAutoCleanup {
    fn name(&self) -> &'static str {
        "use_auto_cleanup"
    }

    fn description(&self) -> &'static str {
        "Suggest g_autoptr/g_autofree/g_autolist instead of manual cleanup"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/use_auto_cleanup.md"))
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: LazyLock<Vec<ConfigOption>> = LazyLock::new(|| {
            vec![ConfigOption {
                name: "ignore_types",
                option_type: "array<string>",
                default_value: "[]",
                example_value: "[\"cairo_*\", \"Pango*\", \"RsvgHandle\"]",
                description: "List of glob patterns for types to ignore",
            }]
        });

        &OPTIONS
    }

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 44))
    }

    fn requires_auto_cleanup(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let ignore_types = self.build_ignore_types_matcher(config);
        for (path, file) in ast_context.iter_c_files() {
            for func in file.iter_function_definitions() {
                self.check_function(func, path, violations, &ignore_types);
                self.check_goto_cleanup(func, file, violations);
            }
        }
    }
}

impl UseAutoCleanup {
    fn build_ignore_types_matcher(&self, config: &Config) -> GlobSet {
        let mut builder = GlobSetBuilder::new();

        for s in config.get_string_list(self.name(), "ignore_types") {
            if let Ok(glob) = Glob::new(&s) {
                builder.add(glob);
            }
        }

        builder.build().unwrap_or_else(|_| GlobSet::empty())
    }

    fn check_function(
        &self,
        func: &FunctionDefItem,
        file_path: &std::path::Path,
        violations: &mut Vec<Violation>,
        ignore_types: &GlobSet,
    ) {
        let local_vars: HashMap<&str, (&TypeInfo, SourceLocation)> = func
            .iter_local_declarations()
            .filter(|d| {
                !d.type_info.uses_auto_cleanup()
                    && d.type_info.is_pointer()
                    && d.is_simple_identifier()
            })
            .map(|d| (d.name.as_str(), (&d.type_info, d.location)))
            .collect();

        for (var_name, (type_info, location)) in &local_vars {
            if let Some(suggestion) = self.suggest_auto_cleanup(func, var_name, type_info) {
                if ignore_types.is_match(&type_info.base_type) {
                    continue;
                }

                violations.push(self.violation(
                    file_path,
                    location.line,
                    location.column,
                    suggestion,
                ));
            }
        }
    }

    fn suggest_auto_cleanup(
        &self,
        func: &FunctionDefItem,
        var_name: &str,
        type_info: &TypeInfo,
    ) -> Option<String> {
        let is_returned = func.is_var_returned(type_info);

        // GError → g_autoptr(GError)
        if type_info.is_base_type("GError")
            && func.is_var_passed_to_function(var_name, "g_error_free", 0)
        {
            return Some(format!(
                "Consider using g_autoptr(GError) {} instead of manual g_error_free",
                var_name
            ));
        }

        // GList/GSList → g_autolist/g_autoslist
        if matches!(type_info.base_type.as_str(), "GList" | "GSList") {
            let free_func = if type_info.base_type == "GList" {
                "g_list_free_full"
            } else {
                "g_slist_free_full"
            };

            if func.is_var_passed_to_function(var_name, free_func, 0)
                && !self.uses_basic_destructor(func, free_func)
                && !is_returned
            {
                let (auto_type, base_type) = match type_info.base_type.as_str() {
                    "GList" => ("g_autolist", "g_list"),
                    _ => ("g_autoslist", "g_slist"),
                };
                return Some(format!(
                    "Consider using {auto_type} to avoid manual {base_type}_free_full cleanup",
                ));
            }
            return None;
        }

        // g_free'd with autofree-suitable allocation → g_autofree
        let is_freed_with_g_free = func.is_var_passed_to_function(var_name, "g_free", 0);
        if is_freed_with_g_free {
            let is_autofree_allocated = func.is_var_allocated_with(type_info, |call| {
                call.function_name_str()
                    .is_some_and(|name| AUTOFREE_ALLOCATIONS.contains(&name))
            });

            if is_autofree_allocated && !is_returned {
                return Some(format!(
                    "Consider using g_autofree {} to avoid manual g_free",
                    var_name
                ));
            }
            return None;
        }

        // g_ptr_array_free(array, FALSE) / g_array_free(array, FALSE) return the
        // element data -> Skip
        if self.frees_array_keeping_data(func, var_name, type_info) {
            return None;
        }

        // General case: allocated + manually freed + not returned → g_autoptr(Type)
        let is_allocated = func.is_var_allocated(type_info);
        let is_manually_freed = func.is_var_passed_to_cleanup(type_info);

        if is_allocated && is_manually_freed && !is_returned {
            return Some(format!(
                "Consider using g_autoptr({}) {} to avoid manual cleanup",
                type_info.base_type, var_name
            ));
        }

        None
    }

    fn frees_array_keeping_data(
        &self,
        func: &FunctionDefItem,
        var_name: &str,
        type_info: &TypeInfo,
    ) -> bool {
        let free_func = match type_info.base_type.as_str() {
            "GPtrArray" => "g_ptr_array_free",
            "GArray" => "g_array_free",
            _ => return false,
        };

        let calls = func.find_calls(&[free_func]);
        for call in calls {
            if call
                .get_arg(0)
                .is_some_and(|arg| matches!(arg, Expression::Identifier(id) if id.name == var_name))
                && call.arguments.len() >= 2
            {
                let Argument::Expression(expr) = &call.arguments[1];
                if expr.is_falsy() {
                    return true;
                }
            }
        }

        false
    }

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

    fn check_goto_cleanup(
        &self,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let allocated_vars = self.find_allocated_variables(&func.body_statements);
        let goto_labels = self.find_goto_labels(&func.body_statements);
        let cleanup_labels = self.find_cleanup_labels(&func.body_statements, &file.source);

        for (var_name, (type_info, location)) in &allocated_vars {
            for goto_label in &goto_labels {
                if let Some(cleanup_vars) = cleanup_labels.get(goto_label)
                    && cleanup_vars.contains(*var_name)
                {
                    violations.push(self.violation(
                        &file.path,
                        location.line,
                        location.column,
                        format!(
                            "Consider using g_autoptr({}) {} and g_steal_pointer to avoid goto cleanup",
                            type_info.base_type, var_name
                        ),
                    ));
                }
            }
        }
    }

    fn find_allocated_variables<'a>(
        &self,
        statements: &'a [Statement],
    ) -> HashMap<&'a str, (&'a TypeInfo, SourceLocation)> {
        let mut result = HashMap::new();

        let local_vars: HashMap<&str, (&TypeInfo, SourceLocation)> = statements
            .iter()
            .flat_map(Statement::iter_declarations)
            .filter(|d| {
                !d.type_info.uses_auto_cleanup()
                    && d.type_info.is_pointer()
                    && d.is_simple_identifier()
            })
            .map(|d| (d.name.as_str(), (&d.type_info, d.location)))
            .collect();

        self.collect_allocated_vars(statements, &local_vars, &mut result);

        result
    }

    fn collect_allocated_vars<'a>(
        &self,
        statements: &'a [Statement],
        local_vars: &HashMap<&str, (&'a TypeInfo, SourceLocation)>,
        result: &mut HashMap<&'a str, (&'a TypeInfo, SourceLocation)>,
    ) {
        for stmt in statements {
            stmt.walk(&mut |s| match s {
                Statement::Declaration(decl) => {
                    if let Some(Expression::Call(call)) = &decl.initializer
                        && call.is_allocation_call()
                        && let Some((type_info, location)) = local_vars.get(decl.name.as_str())
                    {
                        result.insert(decl.name.as_str(), (*type_info, *location));
                    }
                }
                Statement::Expression(expr_stmt) => {
                    if let Expression::Assignment(assign) = expr_stmt.as_ref()
                        && let Expression::Call(call) = &*assign.rhs
                        && call.is_allocation_call()
                        && let Expression::Identifier(id) = &*assign.lhs
                        && let Some((type_info, location)) = local_vars.get(id.name.as_str())
                    {
                        result.insert(id.name.as_str(), (*type_info, *location));
                    }
                }
                _ => {}
            });
        }
    }

    fn find_goto_labels<'a>(
        &self,
        statements: &'a [Statement],
    ) -> std::collections::HashSet<&'a str> {
        let mut labels = std::collections::HashSet::new();
        for stmt in statements {
            stmt.walk(&mut |s| {
                if let Statement::Goto(goto_stmt) = s {
                    labels.insert(goto_stmt.label.as_str());
                }
            });
        }
        labels
    }

    fn find_cleanup_labels<'a>(
        &self,
        statements: &'a [Statement],
        source: &'a [u8],
    ) -> HashMap<&'a str, std::collections::HashSet<&'a str>> {
        let mut result = HashMap::new();

        for stmt in statements {
            stmt.walk(&mut |s| {
                if let Statement::Labeled(labeled) = s {
                    let cleanup_vars = self.find_cleanup_calls(&labeled.statement, source);
                    if !cleanup_vars.is_empty() {
                        result.insert(labeled.label.as_str(), cleanup_vars);
                    }
                }
            });
        }

        result
    }

    fn find_cleanup_calls<'a>(
        &self,
        stmt: &Statement,
        source: &'a [u8],
    ) -> std::collections::HashSet<&'a str> {
        let mut cleanup_vars = std::collections::HashSet::new();
        for call in stmt.iter_calls() {
            if call.is_cleanup_call()
                && let Some(arg_expr) = call.get_arg(0)
                && let Some(var_name) = arg_expr.extract_variable_name(source)
            {
                cleanup_vars.insert(var_name);
            }
        }
        cleanup_vars
    }
}
