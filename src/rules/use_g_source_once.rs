use gobject_ast::model::{
    BasicType, Expression, FileModel, FunctionDeclItem, FunctionDefItem, Statement,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

fn gsource_callback_arg_index(func_name: &str) -> Option<usize> {
    match func_name {
        "g_idle_add" => Some(0),
        "g_timeout_add" | "g_timeout_add_seconds" => Some(1),
        _ => None,
    }
}

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

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 74))
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        // Find g_idle_add, g_timeout_add, and g_timeout_add_seconds calls
        for call in func.find_calls(&["g_idle_add", "g_timeout_add", "g_timeout_add_seconds"]) {
            let Some(idx) = gsource_callback_arg_index(call.function_name_str().unwrap_or(""))
            else {
                continue;
            };
            if let Some(callback_name) = call.get_arg(idx).and_then(|a| a.extract_identifier_name())
            {
                // Only proceed if callback is NOT used elsewhere
                if !self.is_callback_used_elsewhere(callback_name, file) {
                    // Find the callback function definition and check if all returns are
                    // FALSE/G_SOURCE_REMOVE
                    if let Some(callback_fixes) = self.get_callback_fixes(callback_name, file) {
                        let func_name = call.function_name();
                        let replacement = match func_name {
                            "g_idle_add" => "g_idle_add_once",
                            "g_timeout_add_seconds" => "g_timeout_add_seconds_once",
                            _ => "g_timeout_add_once",
                        };

                        // Build arguments, replacing GSourceFunc cast with GSourceOnceFunc if
                        // present
                        let args_str = call
                            .arguments
                            .iter()
                            .enumerate()
                            .filter_map(|(i, arg)| {
                                if i == idx {
                                    // Callback argument - replace cast type if present
                                    if let Expression::Cast(cast) = &**arg
                                        && let Some(callback_name) =
                                            cast.operand.location().as_str()
                                    {
                                        return Some(format!(
                                            "(GSourceOnceFunc) {}",
                                            callback_name
                                        ));
                                    }
                                }
                                arg.location().as_str().map(ToOwned::to_owned)
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        // Fix 1: Replace g_idle_add → g_idle_add_once
                        let arg_refs: Vec<&str> = args_str.split(", ").collect();
                        let mut fixes = vec![Fix::new(
                            call.location.start_byte,
                            call.location.end_byte,
                            config.style.format_call(replacement, &arg_refs),
                        )];

                        // Add callback fixes (return type + return statements)
                        fixes.extend(callback_fixes);

                        violations.push(self.violation_with_fixes_at(
                            &file.path,
                            &call.location,
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
    fn get_callback_fixes(&self, callback_name: &str, file: &FileModel) -> Option<Vec<Fix>> {
        let mut fixes = Vec::new();
        let mut found_definition = false;

        for func in file.iter_function_definitions() {
            if func.name != callback_name {
                continue;
            }

            let return_stmts: Vec<_> = func
                .body_statements
                .iter()
                .flat_map(Statement::iter_returns)
                .collect();
            if return_stmts.is_empty() {
                return None;
            }

            if !return_stmts.iter().all(|ret| {
                ret.value.as_ref().is_some_and(|expr| {
                    expr.is_falsy()
                        || matches!(expr, Expression::Identifier(id) if id.name == "G_SOURCE_REMOVE")
                })
            }) {
                return None;
            }

            if let Some(fix) = self.fix_definition_return_type(func) {
                fixes.push(fix);
            }

            let last_top_level = func.body_statements.last().and_then(|s| {
                if let Statement::Return(ret) = s {
                    Some(&ret.location)
                } else {
                    None
                }
            });

            for ret in &return_stmts {
                if last_top_level.is_some_and(|loc| loc.start_byte == ret.location.start_byte) {
                    fixes.push(Fix::delete_line_and_leading_blank(&ret.location));
                } else {
                    fixes.push(Fix::new(
                        ret.location.start_byte,
                        ret.location.end_byte,
                        "return;",
                    ));
                }
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

    fn fix_definition_return_type(&self, func: &FunctionDefItem) -> Option<Fix> {
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

    fn fix_declaration_return_type(&self, func: &FunctionDeclItem) -> Option<Fix> {
        // Check if return type is gboolean
        if func.return_type.as_basic() != Some(BasicType::Boolean) {
            return None;
        }

        // Preserve alignment by padding "void" to match the original type length
        let replacement = format!(
            "{:width$}",
            "void",
            width = func.return_type.display_name().len()
        );

        // Use the location from the return type's TypeInfo
        Some(Fix::new(
            func.return_type.location.start_byte,
            func.return_type.location.end_byte,
            replacement,
        ))
    }

    fn is_callback_used_elsewhere(&self, callback_name: &str, file: &FileModel) -> bool {
        for func in file.iter_function_definitions() {
            if self.has_non_source_add_usage(&func.body_statements, callback_name) {
                return true;
            }
        }

        false
    }

    fn has_non_source_add_usage(&self, statements: &[Statement], callback_name: &str) -> bool {
        for stmt in statements {
            let mut found = false;
            stmt.walk(&mut |s| {
                if !self.is_source_add_statement(s, callback_name) {
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

    fn is_source_add_statement(&self, stmt: &Statement, callback_name: &str) -> bool {
        if let Statement::Expression(expr_stmt) = stmt
            && let Expression::Call(call) = expr_stmt.as_ref()
            && let Some(idx) = gsource_callback_arg_index(call.function_name_str().unwrap_or(""))
            && let Some(name) = call.get_arg(idx).and_then(|a| a.extract_identifier_name())
        {
            name == callback_name
        } else {
            false
        }
    }
}
