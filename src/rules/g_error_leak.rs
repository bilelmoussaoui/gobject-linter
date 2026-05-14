use std::sync::LazyLock;

use gobject_ast::model::{Argument, Expression, FileModel, FunctionDefItem, Statement, UnaryOp};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, ConfigOption, Rule, Violation},
};

pub struct GErrorLeak;

impl Rule for GErrorLeak {
    fn name(&self) -> &'static str {
        "g_error_leak"
    }

    fn description(&self) -> &'static str {
        "Check for GError variables that are neither freed nor propagated"
    }

    fn category(&self) -> Category {
        Category::Correctness
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: LazyLock<Vec<ConfigOption>> = LazyLock::new(|| {
            vec![
                ConfigOption {
                    name: "extra_noreturn_functions",
                    option_type: "array<string>",
                    default_value: "[]",
                    example_value: "[\"my_app_abort\", \"test_fail\"]",
                    description: "Additional function names that never return (terminate the program), suppressing leak warnings",
                },
                ConfigOption {
                    name: "extra_propagation_functions",
                    option_type: "array<string>",
                    default_value: "[]",
                    example_value: "[\"my_app_report_error\", \"dbus_reply_error\"]",
                    description: "Additional function names that take ownership of the GError (propagation/transfer)",
                },
            ]
        });
        &OPTIONS
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let extra_noreturn = config.get_string_list(self.name(), "extra_noreturn_functions");
        let extra_propagation = config.get_string_list(self.name(), "extra_propagation_functions");

        let mut gerror_vars = Vec::new();

        for stmt in &func.body_statements {
            for decl in stmt.iter_declarations() {
                if decl.type_info.is_base_type("GError")
                    && decl.type_info.is_pointer()
                    && decl.initializer.as_ref().is_some_and(Expression::is_null)
                {
                    gerror_vars.push((decl.name.as_str(), &decl.location));
                }
            }
        }

        for (var_name, loc) in gerror_vars {
            let is_used = is_error_used(&func.body_statements, var_name);

            if !is_used {
                continue;
            }

            let is_freed = is_error_freed(&func.body_statements, var_name);
            let is_propagated =
                is_error_propagated(&func.body_statements, var_name, &extra_propagation);
            let has_noreturn = calls_noreturn_function(&func.body_statements, &extra_noreturn);

            if !is_freed && !is_propagated && !has_noreturn {
                violations.push(self.violation_at(
                    &file.path,
                    loc,
                    format!(
                        "GError variable '{}' may be leaked; it should be freed with g_error_free/g_clear_error or propagated with g_propagate_error/g_task_return_error/g_steal_pointer",
                        var_name
                    ),
                ));
            }
        }
    }
}

fn calls_noreturn_function(statements: &[Statement], extra: &[String]) -> bool {
    const BUILTIN: &[&str] = &[
        "g_error",
        "g_assert",
        "g_assert_not_reached",
        "g_assert_no_error",
        "g_return_if_fail",
        "g_return_val_if_fail",
        "exit",
        "abort",
        "_exit",
    ];

    for stmt in statements {
        for call in stmt.iter_calls() {
            if let Some(func_name) = call.function_name_str()
                && (BUILTIN.contains(&func_name) || extra.iter().any(|e| e == func_name))
            {
                return true;
            }
        }
    }
    false
}

/// Check if the error variable is used (passed to functions as &error)
fn is_error_used(statements: &[Statement], var_name: &str) -> bool {
    for stmt in statements {
        let mut found = false;
        stmt.walk_expressions(&mut |expr| {
            // Recursively walk ALL nested expressions
            expr.walk(&mut |nested_expr| {
                // Check for &error pattern (address-of operator)
                if let Expression::Unary(unary) = nested_expr
                    && unary.operator == UnaryOp::AddressOf
                    && let Expression::Identifier(id) = &*unary.operand
                    && id.name == var_name
                {
                    found = true;
                }
            });
        });
        if found {
            return true;
        }
    }
    false
}

/// Check if the error variable is freed (g_error_free or g_clear_error)
fn is_error_freed(statements: &[Statement], var_name: &str) -> bool {
    check_error_handled(statements, var_name, &["g_error_free", "g_clear_error"])
}

/// Check if the error variable is propagated (g_propagate_error,
/// g_steal_pointer, g_task_return_error, etc.)
fn is_error_propagated(statements: &[Statement], var_name: &str, extra: &[String]) -> bool {
    // Check for known ownership-transfer functions
    if check_error_handled(
        statements,
        var_name,
        &[
            "g_propagate_error",
            "g_propagate_prefixed_error",
            "g_steal_pointer",
            "g_task_return_error",
            "g_dbus_method_invocation_take_error",
        ],
    ) {
        return true;
    }

    if !extra.is_empty() && check_error_handled(statements, var_name, extra) {
        return true;
    }

    for stmt in statements {
        for call in stmt.iter_calls() {
            if let Some(func_name) = call.function_name_str()
                && (func_name.contains("_terminate_") && func_name.contains("error")
                    || func_name.ends_with("_set_error")
                    || func_name.contains("_set_g_error"))
            {
                for arg in &call.arguments {
                    let Argument::Expression(arg_expr) = arg;
                    if arg_expr.contains_identifier(var_name) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn check_error_handled<S: AsRef<str>>(
    statements: &[Statement],
    var_name: &str,
    functions: &[S],
) -> bool {
    for stmt in statements {
        for call in stmt.iter_calls() {
            if let Some(func_name) = call.function_name_str()
                && functions.iter().any(|f| f.as_ref() == func_name)
            {
                for arg in &call.arguments {
                    let Argument::Expression(arg_expr) = arg;
                    if arg_expr.contains_identifier(var_name) {
                        return true;
                    }
                }
            }
        }
    }
    false
}
