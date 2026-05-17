use std::sync::LazyLock;

use gobject_ast::model::{Expression, FileModel, FunctionDefItem, Statement};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{ConfigOption, Rule, Violation},
};

pub struct GSourceIdNotStored;

const SOURCE_FUNCTIONS: &[&str] = &[
    "g_timeout_add",
    "g_timeout_add_full",
    "g_timeout_add_seconds",
    "g_timeout_add_seconds_full",
    "g_idle_add",
    "g_idle_add_full",
    "gtk_widget_add_tick_callback",
];

const ONCE_SOURCE_FUNCTIONS: &[&str] = &[
    "g_timeout_add_once",
    "g_timeout_add_seconds_once",
    "g_idle_add_once",
];

impl Rule for GSourceIdNotStored {
    fn name(&self) -> &'static str {
        "g_source_id_not_stored"
    }

    fn description(&self) -> &'static str {
        "Warn when GSource timeout/idle functions are called without storing the returned ID"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Suspicious
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: LazyLock<Vec<ConfigOption>> = LazyLock::new(|| {
            vec![ConfigOption {
                name: "check_once_functions",
                option_type: "bool",
                default_value: "true",
                example_value: "false",
                description: "Whether to check _once variants (g_idle_add_once, g_timeout_add_once, etc.)",
            }]
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
        let with_once = config
            .get_rule_config(self.name())
            .and_then(|rc| rc.options.get("check_once_functions"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);

        for stmt in &func.body_statements {
            stmt.walk(&mut |s| {
                if let Statement::Expression(expr_stmt) = s
                    && (expr_stmt.is_call_to_any(SOURCE_FUNCTIONS)
                        || (with_once && expr_stmt.is_call_to_any(ONCE_SOURCE_FUNCTIONS)))
                    && let Expression::Call(call) = expr_stmt.as_ref()
                    && !call.arguments.is_empty()
                        && call.has_arg_matching(call.arguments.len() - 1, |expr| !expr.is_null())
                    {
                        violations.push(self.violation_at(
                            &file.path,
                            &call.location,
                            format!(
                                "{}() called without storing the returned source ID. If the object is destroyed before the callback fires, this will cause a use-after-free. Store the ID and use g_clear_handle_id() in dispose.",
                                call.function_name()
                            ),
                        ));
                    }
            });
        }
    }
}
