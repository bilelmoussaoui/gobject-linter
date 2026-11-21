use std::{collections::HashMap, sync::LazyLock};

use globset::{Glob, GlobSet, GlobSetBuilder};
use gobject_ast::model::{
    Expression, FileModel, FunctionDefItem, SourceLocation, Statement, TypeInfo,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{ConfigOption, Rule, Violation},
};

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
            vec![
                ConfigOption {
                    name: "ignore_types",
                    option_type: "array<string>",
                    default_value: "[]",
                    example_value: "[\"cairo_*\", \"Pango*\", \"RsvgHandle\"]",
                    description: "List of glob patterns for types to ignore",
                },
                ConfigOption {
                    name: "allocation_proof",
                    option_type: "bool",
                    default_value: "true",
                    example_value: "false",
                    description: "Only suggest auto-cleanup when the variable is provably allocated in the current function. When false, any manually freed variable is flagged.",
                },
            ]
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
        let allocation_proof = config
            .get_rule_config(self.name())
            .and_then(|rc| rc.options.get("allocation_proof"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);
        for (path, file) in ast_context.iter_c_files() {
            for func in file.iter_function_definitions() {
                self.check_function(func, path, violations, &ignore_types, allocation_proof);
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
        allocation_proof: bool,
    ) {
        let local_vars: HashMap<&str, (&TypeInfo, &SourceLocation)> = func
            .iter_local_declarations()
            .filter(|d| {
                !d.type_info.uses_auto_cleanup()
                    && (d.type_info.pointer_depth == 1 || Self::is_strv_type(&d.type_info))
                    && d.is_simple_identifier()
            })
            .map(|d| (d.name.as_str(), (&d.type_info, &d.location)))
            .collect();

        for (var_name, (type_info, location)) in &local_vars {
            if let Some(suggestion) =
                self.suggest_auto_cleanup(func, var_name, type_info, allocation_proof)
            {
                if ignore_types.is_match(&type_info.base_type) {
                    continue;
                }

                violations.push(self.violation_at(file_path, location, suggestion));
            }
        }
    }

    fn suggest_auto_cleanup(
        &self,
        func: &FunctionDefItem,
        var_name: &str,
        type_info: &TypeInfo,
        allocation_proof: bool,
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

        // GStrv / gchar** → g_auto(GStrv)
        if Self::is_strv_type(type_info)
            && func.is_var_passed_to_function(var_name, "g_strfreev", 0)
            && (!allocation_proof || func.is_named_var_allocated(var_name))
            && !is_returned
        {
            return Some(format!(
                "Consider using g_auto(GStrv) {} to avoid manual g_strfreev",
                var_name
            ));
        }

        // g_free'd with a recognized allocation → g_autofree
        let is_freed_with_g_free = func.is_var_passed_to_function(var_name, "g_free", 0);
        if is_freed_with_g_free {
            if (!allocation_proof || func.is_named_var_allocated(var_name)) && !is_returned {
                return Some(format!(
                    "Consider using g_autofree {} to avoid manual g_free",
                    var_name
                ));
            }
            return None;
        }

        // g_ptr_array_free(array, FALSE) / g_array_free(array, FALSE) return
        // the element data -> Skip
        if self.frees_array_keeping_data(func, var_name, type_info) {
            return None;
        }

        // General case: allocated + manually freed + not returned →
        // g_autoptr(Type)
        let is_allocated = !allocation_proof || func.is_named_var_allocated(var_name);
        let is_manually_freed = func.is_named_var_passed_to_cleanup(var_name);

        if is_allocated && is_manually_freed && !is_returned {
            return Some(format!(
                "Consider using g_autoptr({}) {} to avoid manual cleanup",
                type_info.base_type, var_name
            ));
        }

        None
    }

    fn is_strv_type(type_info: &TypeInfo) -> bool {
        (matches!(type_info.base_type.as_str(), "gchar" | "char") && type_info.pointer_depth == 2)
            || (type_info.base_type == "GStrv" && type_info.pointer_depth == 0)
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
                && call.arguments[1].is_falsy()
            {
                return true;
            }
        }

        false
    }

    fn uses_basic_destructor(&self, func: &FunctionDefItem, free_func: &str) -> bool {
        let calls = func.find_calls(&[free_func]);

        for call in calls {
            if call.arguments.len() >= 2
                && let Expression::Identifier(destructor) = call.arguments[1].as_ref()
                && matches!(
                    destructor.name.as_str(),
                    "g_free" | "free" | "g_slice_free" | "g_slice_free1"
                )
            {
                return true;
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
        let cleanup_labels = self.find_cleanup_labels(&func.body_statements);

        for (var, (type_info, location)) in &allocated_vars {
            for goto_label in &goto_labels {
                if let Some(cleanup_vars) = cleanup_labels.get(goto_label)
                    && cleanup_vars.contains(var)
                {
                    violations.push(self.violation_at(
                        &file.path,
                        location,
                        format!(
                            "Consider using g_autoptr({}) {} and g_steal_pointer to avoid goto cleanup",
                            type_info.base_type, var.location().as_str().unwrap_or_default()
                        ),
                    ));
                }
            }
        }
    }

    fn find_allocated_variables<'a>(
        &self,
        statements: &'a [Statement],
    ) -> HashMap<Expression, (&'a TypeInfo, &'a SourceLocation)> {
        let mut result = HashMap::new();

        let local_vars: HashMap<Expression, (&TypeInfo, &SourceLocation)> = statements
            .iter()
            .flat_map(Statement::iter_declarations)
            .filter(|d| {
                !d.type_info.uses_auto_cleanup()
                    && d.type_info.is_pointer()
                    && d.is_simple_identifier()
            })
            .map(|d| (d.as_expression(), (&d.type_info, &d.location)))
            .collect();

        self.collect_allocated_vars(statements, &local_vars, &mut result);

        result
    }

    fn collect_allocated_vars<'a>(
        &self,
        statements: &'a [Statement],
        local_vars: &HashMap<Expression, (&'a TypeInfo, &'a SourceLocation)>,
        result: &mut HashMap<Expression, (&'a TypeInfo, &'a SourceLocation)>,
    ) {
        for stmt in statements {
            stmt.walk(&mut |s| match s {
                Statement::Declaration(decl) => {
                    if let Some(Expression::Call(call)) = &decl.initializer
                        && call.is_allocation_call()
                    {
                        let var = decl.as_expression();
                        if let Some((type_info, location)) = local_vars.get(&var) {
                            result.insert(var, (*type_info, location));
                        }
                    }
                }
                Statement::Expression(expr_stmt) => {
                    if let Expression::Assignment(assign) = expr_stmt.as_ref()
                        && let Expression::Call(call) = &*assign.rhs
                        && call.is_allocation_call()
                        && matches!(assign.lhs.as_ref(), Expression::Identifier(_))
                        && let Some((type_info, location)) = local_vars.get(assign.lhs.as_ref())
                    {
                        result.insert(assign.lhs.as_ref().clone(), (*type_info, location));
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
        &'a self,
        statements: &'a [Statement],
    ) -> HashMap<&'a str, std::collections::HashSet<&'a Expression>> {
        let mut result = HashMap::new();

        for stmt in statements {
            stmt.walk(&mut |s| {
                if let Statement::Labeled(labeled) = s {
                    let cleanup_vars = self.find_cleanup_calls(&labeled.statement);
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
        stmt: &'a Statement,
    ) -> std::collections::HashSet<&'a Expression> {
        let mut cleanup_vars = std::collections::HashSet::new();
        for call in stmt.iter_calls() {
            if call.is_cleanup_call()
                && let Some(arg_expr) = call.get_arg(0)
                && let Some(var) = arg_expr.extract_variable()
            {
                cleanup_vars.insert(var);
            }
        }
        cleanup_vars
    }
}
