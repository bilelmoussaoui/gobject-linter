use gobject_ast::Expression;

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGNew;

impl Rule for UseGNew {
    fn name(&self) -> &'static str {
        "use_g_new"
    }

    fn description(&self) -> &'static str {
        "Suggest g_new/g_new0 instead of g_malloc/g_malloc0 with sizeof for type safety"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for call in func.find_calls(&["g_malloc", "g_malloc0"]) {
            self.check_call(config, file, call, violations);
        }
    }
}

impl UseGNew {
    fn check_call(
        &self,
        config: &Config,
        file: &gobject_ast::FileModel,

        call: &gobject_ast::CallExpression,
        violations: &mut Vec<Violation>,
    ) {
        // Need exactly 1 argument
        if call.arguments.len() != 1 {
            return;
        }

        // Check if argument is sizeof(Type)
        let Some(arg_expr) = call.get_arg(0) else {
            return;
        };
        let Expression::Sizeof(sizeof_expr) = arg_expr else {
            return;
        };

        // Extract the type - only works for simple types/identifiers
        let Some(type_name) = sizeof_expr.type_name() else {
            // Complex expression, not a simple type - skip
            return;
        };

        let func_name = call.function_name(&file.source);
        let suggested_func = if call.is_function("g_malloc0") {
            "g_new0"
        } else {
            "g_new"
        };

        let replacement = config.format_call(suggested_func, &[type_name, "1"]);
        let message = format!(
            "Use {} instead of {}(sizeof({})) for type safety",
            replacement, func_name, type_name
        );
        let fix = Fix::new(
            call.location.start_byte,
            call.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix(
            &file.path,
            call.location.line,
            call.location.column,
            message,
            fix,
        ));
    }
}
