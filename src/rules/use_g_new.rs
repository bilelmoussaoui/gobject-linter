use std::collections::HashMap;

use gobject_ast::model::{
    CallExpression, Expression, FileModel, FunctionDefItem, SizeofOperand, TypeInfo, UnaryOp,
};

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

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/use_g_new.md"))
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
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        let var_types = func.local_var_types();
        for call in func.find_calls(&["g_malloc", "g_malloc0"]) {
            self.check_call(file, call, &var_types, config, violations);
        }
    }
}

impl UseGNew {
    fn check_call(
        &self,
        file: &FileModel,
        call: &CallExpression,
        var_types: &HashMap<&str, &TypeInfo>,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() != 1 {
            return;
        }

        let Some(arg_expr) = call.get_arg(0) else {
            return;
        };
        let Expression::Sizeof(sizeof_expr) = arg_expr else {
            return;
        };

        let resolved_type;
        let type_name = match &sizeof_expr.operand {
            Some(SizeofOperand::Type(t)) => {
                resolved_type = t.qualified_base_name();
                resolved_type.as_str()
            }
            Some(SizeofOperand::Expression(expr)) => {
                if let Expression::Identifier(id) = expr.as_ref() {
                    if let Some(type_info) = var_types.get(id.name.as_str()) {
                        if type_info.pointer_depth > 0 {
                            return;
                        }
                        resolved_type = type_info.qualified_base_name();
                        resolved_type.as_str()
                    } else if Self::looks_like_macro_constant(&id.name) {
                        return;
                    } else {
                        id.name.as_str()
                    }
                } else if let Expression::Unary(unary) = expr.as_ref()
                    && unary.operator == UnaryOp::Dereference
                    && let Expression::Identifier(id) = unary.operand.as_ref()
                {
                    if let Some(type_info) = var_types.get(id.name.as_str()) {
                        if type_info.pointer_depth < 1 {
                            return;
                        }
                        let base = type_info.qualified_base_name();
                        let deref_depth = type_info.pointer_depth - 1;
                        if deref_depth > 0 {
                            resolved_type = format!("{} {}", base, "*".repeat(deref_depth));
                        } else {
                            resolved_type = base;
                        }
                        resolved_type.as_str()
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
            None => return,
        };

        let func_name = call.function_name();
        let suggested_func = if call.is_function("g_malloc0") {
            "g_new0"
        } else {
            "g_new"
        };

        let replacement = config.style.format_call(suggested_func, &[type_name, "1"]);
        let message = format!(
            "Use {} instead of {}(sizeof({})) for type safety",
            replacement, func_name, type_name
        );
        let fix = Fix::new(
            call.location.start_byte,
            call.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix_at(&file.path, &call.location, message, fix));
    }

    fn looks_like_macro_constant(name: &str) -> bool {
        name.len() > 1
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
    }
}
