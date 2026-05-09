use gobject_ast::{Expression, Statement, model::types::BasicType};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGSourceOnce;

impl Rule for UseGSourceOnce {
    fn name(&self) -> &'static str {
        "use_g_source_once"
    }

    fn description(&self) -> &'static str {
        "Suggest using g_idle_add_once/g_timeout_add_once/g_timeout_add_seconds_once when callback always returns G_SOURCE_REMOVE"
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
        _config: &Config,
        func: &gobject_ast::types::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Find g_idle_add, g_timeout_add, and g_timeout_add_seconds calls
        for call in func.find_calls(&["g_idle_add", "g_timeout_add", "g_timeout_add_seconds"]) {
            // Get the callback name from the first argument
            if let Some(callback_name) = self.extract_callback_name(call, &file.source) {
                // Only proceed if callback is NOT used elsewhere
                if !self.is_callback_used_elsewhere(callback_name, file, &file.source) {
                    // Find the callback function definition and check if all returns are
                    // FALSE/G_SOURCE_REMOVE
                    if let Some(callback_fixes) = self.get_callback_fixes(callback_name, file) {
                        let func_name = call.function_name(&file.source);
                        let replacement = match func_name {
                            "g_idle_add" => "g_idle_add_once",
                            "g_timeout_add_seconds" => "g_timeout_add_seconds_once",
                            _ => "g_timeout_add_once",
                        };

                        // Determine callback argument index
                        let callback_arg_index = if func_name == "g_idle_add" { 0 } else { 1 };

                        // Build arguments, replacing GSourceFunc cast with GSourceOnceFunc if
                        // present
                        let args_str = call
                            .arguments
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, arg)| {
                                if idx == callback_arg_index {
                                    // Callback argument - replace cast type if present
                                    let gobject_ast::Argument::Expression(expr) = arg;
                                    if let Expression::Cast(cast) = &**expr
                                        && let Some(callback_name) =
                                            cast.operand.to_source_string(&file.source)
                                    {
                                        return Some(format!(
                                            "(GSourceOnceFunc) {}",
                                            callback_name
                                        ));
                                    }
                                }
                                arg.to_source_string(&file.source).map(ToOwned::to_owned)
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        // Fix 1: Replace g_idle_add → g_idle_add_once
                        let mut fixes = vec![Fix::new(
                            call.location.start_byte,
                            call.location.end_byte,
                            format!("{} ({})", replacement, args_str),
                        )];

                        // Add callback fixes (return type + return statements)
                        fixes.extend(callback_fixes);

                        violations.push(self.violation_with_fixes(
                            &file.path,
                            call.location.line,
                            call.location.column,
                            format!(
                                "Callback '{}' always returns G_SOURCE_REMOVE. Use {} instead of {}",
                                callback_name, replacement, func_name
                            ),
                            fixes,
                        ));
                    }
                }
            }
        }
    }
}

impl UseGSourceOnce {
    fn extract_callback_name<'a>(
        &self,
        call: &'a gobject_ast::CallExpression,
        source: &[u8],
    ) -> Option<&'a str> {
        // Determine which argument is the callback based on the function name
        // g_idle_add(callback, user_data) -> arg 0
        // g_timeout_add(interval, callback, user_data) -> arg 1
        // g_timeout_add_seconds(interval, callback, user_data) -> arg 1
        let func_name = call.function_name(source);
        let callback_arg_index = if func_name == "g_idle_add" {
            0
        } else {
            1 // g_timeout_add or g_timeout_add_seconds
        };

        let arg_expr = call.get_arg(callback_arg_index)?;

        // Handle direct identifier
        if let Expression::Identifier(id) = arg_expr {
            return Some(id.name.as_str());
        }

        // Handle casted callback: (GSourceFunc) callback_name
        if let Expression::Cast(cast) = arg_expr
            && let Expression::Identifier(id) = &*cast.operand
        {
            return Some(id.name.as_str());
        }

        None
    }

    fn get_callback_fixes(
        &self,
        callback_name: &str,
        file: &gobject_ast::FileModel,
    ) -> Option<Vec<Fix>> {
        let mut fixes = Vec::new();
        let mut found_definition = false;

        for func in file.iter_function_definitions() {
            if func.name != callback_name {
                continue;
            }

            let return_exprs = func.collect_return_values();
            if return_exprs.is_empty() {
                return None;
            }

            if !return_exprs.iter().all(|expr| {
                expr.to_source_string(&file.source).is_some_and(|s| {
                    s == "FALSE" || s == "G_SOURCE_REMOVE" || s == "0" || s == "false"
                })
            }) {
                return None;
            }

            if let Some(fix) = self.fix_definition_return_type(func) {
                fixes.push(fix);
            }

            for ret_expr in return_exprs {
                let (line_start, line_end) = ret_expr.location().find_line_bounds(&file.source);
                fixes.push(Fix::new(line_start, line_end, String::new()));
            }

            found_definition = true;
        }

        for func in file.iter_function_declarations() {
            if func.name != callback_name {
                continue;
            }
            if let Some(fix) = self.fix_declaration_return_type(func) {
                fixes.push(fix);
            }
        }

        if found_definition && !fixes.is_empty() {
            Some(fixes)
        } else {
            None
        }
    }

    fn fix_definition_return_type(
        &self,
        func: &gobject_ast::types::FunctionDefItem,
    ) -> Option<Fix> {
        // Check if return type is gboolean
        if func.return_type.as_basic() != Some(BasicType::Boolean) {
            return None;
        }

        // Use the location from the return type's TypeInfo
        Some(Fix::new(
            func.return_type.location.start_byte,
            func.return_type.location.end_byte,
            "void".to_string(),
        ))
    }

    fn fix_declaration_return_type(
        &self,
        func: &gobject_ast::types::FunctionDeclItem,
    ) -> Option<Fix> {
        // Check if return type is gboolean
        if func.return_type.as_basic() != Some(BasicType::Boolean) {
            return None;
        }

        // Preserve alignment by padding "void" to match the original type length
        let replacement = format!(
            "{:width$}",
            "void",
            width = func.return_type.full_text.trim().len()
        );

        // Use the location from the return type's TypeInfo
        Some(Fix::new(
            func.return_type.location.start_byte,
            func.return_type.location.end_byte,
            replacement,
        ))
    }

    fn is_callback_used_elsewhere(
        &self,
        callback_name: &str,
        file: &gobject_ast::FileModel,
        source: &[u8],
    ) -> bool {
        for func in file.iter_function_definitions() {
            if self.has_non_source_add_usage(&func.body_statements, callback_name, source) {
                return true;
            }
        }

        false
    }

    fn has_non_source_add_usage(
        &self,
        statements: &[Statement],
        callback_name: &str,
        source: &[u8],
    ) -> bool {
        for stmt in statements {
            let mut found = false;
            stmt.walk(&mut |s| {
                if !self.is_source_add_statement(s, callback_name, source) {
                    s.visit_expressions(&mut |e| {
                        if e.contains_identifier(callback_name) {
                            found = true;
                        }
                    });
                }
            });
            if found {
                return true;
            }
        }
        false
    }

    fn is_source_add_statement(
        &self,
        stmt: &Statement,
        callback_name: &str,
        source: &[u8],
    ) -> bool {
        // Check if this statement is a g_idle_add/g_timeout_add/g_timeout_add_seconds
        // call with our callback
        if let Statement::Expression(expr_stmt) = stmt
            && expr_stmt.is_call_to_any(&["g_idle_add", "g_timeout_add", "g_timeout_add_seconds"])
            && let Expression::Call(call) = expr_stmt.as_ref()
        {
            // Determine which argument is the callback
            let func_name = call.function_name(source);
            let callback_arg_index = if func_name == "g_idle_add" { 0 } else { 1 };

            if let Some(arg_expr) = call.get_arg(callback_arg_index) {
                // Handle direct identifier
                if let Expression::Identifier(id) = arg_expr {
                    return id.name == callback_name;
                }
                // Handle casted callback: (GSourceFunc) callback_name
                if let Expression::Cast(cast) = arg_expr
                    && let Expression::Identifier(id) = &*cast.operand
                {
                    return id.name == callback_name;
                }
            }
        }
        false
    }
}
