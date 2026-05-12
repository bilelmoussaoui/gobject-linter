use gobject_ast::model::{
    CallExpression, EnumInfo, Expression, FileModel, FunctionDefItem, ParamSpecAssignment,
    Statement, TopLevelItem,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGObjectClassInstallProperties;

impl Rule for UseGObjectClassInstallProperties {
    fn name(&self) -> &'static str {
        "use_g_object_class_install_properties"
    }

    fn description(&self) -> &'static str {
        "Suggest g_object_class_install_properties for multiple g_object_class_install_property calls"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn min_glib_version(&self) -> Option<(u32, u32)> {
        Some((2, 26))
    }

    fn check_enum(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        enum_info: &EnumInfo,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if !enum_info.is_property_enum() {
            return;
        }

        let Some(gobject_type) = file.find_gobject_type_for_property_enum(enum_info) else {
            return;
        };

        let class_init_name = gobject_type.class_init_function_name();
        let Some(func) = file
            .iter_function_definitions()
            .find(|f| f.name == class_init_name)
        else {
            return;
        };

        let install_property_calls = func.find_calls(&["g_object_class_install_property"]);
        if install_property_calls.is_empty() {
            return;
        }

        let fixes = self.generate_fixes(
            file,
            func,
            &install_property_calls,
            &gobject_type.properties,
            enum_info,
            &file.source,
            &config.style,
        );

        let first_call = install_property_calls[0];
        let message = if fixes.is_empty() {
            format!(
                "Consider using g_object_class_install_properties() instead of {} g_object_class_install_property() calls",
                install_property_calls.len()
            )
        } else {
            format!(
                "Use g_object_class_install_properties() instead of {} g_object_class_install_property() calls",
                install_property_calls.len()
            )
        };

        violations.push(self.violation_with_fixes_at(
            &file.path,
            &first_call.location,
            message,
            fixes,
        ));
    }
}

impl UseGObjectClassInstallProperties {
    #[allow(clippy::too_many_arguments)]
    fn generate_fixes(
        &self,
        file: &FileModel,
        class_init: &FunctionDefItem,
        install_calls: &[&CallExpression],
        assignments: &[ParamSpecAssignment],
        property_enum: &EnumInfo,
        source: &[u8],
        style: &crate::config::Style,
    ) -> Vec<Fix> {
        let mut fixes = Vec::new();

        // Pre-collect variable-pattern assignments for lookup during fix generation
        let param_spec_assignments: Vec<_> = assignments
            .iter()
            .filter_map(|a| {
                if let ParamSpecAssignment::Variable {
                    variable_name,
                    statement_location,
                    call,
                    ..
                } = a
                {
                    Some((variable_name, statement_location, call))
                } else {
                    None
                }
            })
            .collect();

        // Check if enum has N_PROPS
        let n_props_value = property_enum.values.iter().find(|v| v.is_prop_last());
        let n_props_name = if let Some(n_props) = n_props_value {
            n_props.name.clone()
        } else {
            // Need to add N_PROPS to the enum
            let n_props_name = self.determine_n_props_name(property_enum);

            // Insert N_PROPS after the last enum value
            let last_value = property_enum.values.last().unwrap();
            // Use the same indentation as the last enum value
            let value_indentation = last_value.location.extract_indentation(source);

            // Check if there's a comma at end_byte (some parsers include it, some don't)
            let (insertion_pos, needs_comma) = if last_value.location.end_byte < source.len()
                && source[last_value.location.end_byte] == b','
            {
                // Comma is at end_byte, insert after it
                (last_value.location.end_byte + 1, false)
            } else {
                // No comma at end_byte, we need to add one
                (last_value.location.end_byte, true)
            };

            let n_props_decl = if needs_comma {
                format!(",\n{}{}", value_indentation, n_props_name)
            } else {
                format!("\n{}{}", value_indentation, n_props_name)
            };

            fixes.push(Fix::new(insertion_pos, insertion_pos, n_props_decl));

            n_props_name
        };

        // Determine array name: prefer "props", fallback to "obj_props"
        let array_name = self.determine_array_name(file, source);

        // Fix: Add GParamSpec array declaration after the enum
        // For non-typedef enums, end_byte may point AT the semicolon rather than after
        // it So we need to skip past it if present
        let insertion_pos = if property_enum.location.end_byte < source.len()
            && source[property_enum.location.end_byte] == b';'
        {
            property_enum.location.end_byte + 1
        } else {
            property_enum.location.end_byte
        };

        let array_decl = format!(
            "\n\nstatic GParamSpec *{}[{}] = {{ NULL, }};",
            array_name, n_props_name
        );
        fixes.push(Fix::new(insertion_pos, insertion_pos, array_decl));

        // Find the GObjectClass declaration to get object_class variable name and
        // indentation
        let object_class_var = class_init
            .iter_local_declarations()
            .find(|decl| decl.type_info.base_type == "GObjectClass")
            .map_or("object_class", |decl| decl.name.as_str());

        // Get indentation for the install_properties call
        let indentation = if let Some(first_call) = install_calls.first() {
            if let Some(stmt) = self.find_statement_containing_call(
                &class_init.body_statements,
                first_call.location.start_byte,
            ) {
                stmt.location().extract_indentation(source)
            } else {
                "  ".to_string()
            }
        } else {
            "  ".to_string()
        };

        // Track GParamSpec variable names to delete their declarations later
        let mut param_spec_vars = std::collections::HashSet::new();

        // Convert each g_object_class_install_property call
        for call in install_calls {
            // Extract the property enum value (2nd argument)
            let Some(prop_id_arg) = call.get_arg(1) else {
                continue;
            };
            let Some(prop_id) = prop_id_arg.to_source_string(source) else {
                continue;
            };

            // Extract the g_param_spec call (3rd argument)
            let Some(param_spec_arg) = call.get_arg(2) else {
                continue;
            };

            // Check if this is a variable pattern or direct call
            let (param_spec, delete_install_call) = if let Expression::Call(param_spec_call) =
                param_spec_arg
            {
                // Direct call pattern: g_object_class_install_property(...,
                // g_param_spec_xxx(...))
                let func_name = param_spec_call.function_name(source);
                let new_line_prefix = format!("{}[{}] = {} (", array_name, prop_id, func_name);
                let target_column = indentation.len() + new_line_prefix.len();

                let Some(param_spec_text) = param_spec_arg.to_source_string(source) else {
                    continue;
                };
                (
                    self.reindent_multiline(param_spec_text, target_column),
                    false,
                )
            } else {
                // Variable pattern: param_spec = g_param_spec_xxx(...);
                // g_object_class_install_property(..., param_spec);
                let Some(var_name) = param_spec_arg.to_source_string(source) else {
                    continue;
                };

                // Find the assignment that comes before this install_property call
                let assignment = param_spec_assignments
                    .iter()
                    .filter(|(name, stmt_loc, _)| {
                        name.as_str() == var_name && stmt_loc.start_byte < call.location.start_byte
                    })
                    .max_by_key(|(_, stmt_loc, _)| stmt_loc.start_byte);

                if let Some((_, statement_location, g_param_spec_call)) = assignment {
                    param_spec_vars.insert(var_name);

                    // Use the g_param_spec call from the assignment
                    let func_name = g_param_spec_call.function_name(source);
                    let new_line_prefix = format!("{}[{}] = {} (", array_name, prop_id, func_name);
                    // Note: indentation is not included because it stays in place during
                    // replacement
                    let assignment_indent = statement_location.extract_indentation(source);
                    let target_column = assignment_indent.len() + new_line_prefix.len();

                    let Some(param_spec_text) =
                        Expression::Call((*g_param_spec_call).clone()).to_source_string(source)
                    else {
                        continue;
                    };

                    // Replace the assignment statement with props[PROP_X] = ...
                    let replacement = format!(
                        "{}[{}] = {};",
                        array_name,
                        prop_id,
                        self.reindent_multiline(param_spec_text, target_column)
                    );
                    fixes.push(Fix::new(
                        statement_location.start_byte,
                        statement_location.find_semicolon_end(source),
                        replacement,
                    ));

                    (String::new(), true) // Mark to delete install_property call
                } else {
                    // Fallback - just use the variable name as-is
                    let Some(param_spec_text) = param_spec_arg.to_source_string(source) else {
                        continue;
                    };
                    (param_spec_text.to_owned(), false)
                }
            };

            // Find the statement containing this install_property call
            let Some(stmt) = self.find_statement_containing_call(
                &class_init.body_statements,
                call.location.start_byte,
            ) else {
                continue;
            };

            if delete_install_call {
                // Delete the entire install_property call statement
                fixes.push(Fix::delete_line(stmt.location(), source));
            } else {
                // Replace the statement with array assignment
                let replacement = format!("{}[{}] = {};", array_name, prop_id, param_spec);
                fixes.push(Fix::new(
                    stmt.location().start_byte,
                    stmt.location().find_semicolon_end(source),
                    replacement,
                ));
            }
        }

        // Remove GParamSpec variable declarations
        for var_name in param_spec_vars {
            if let Some(decl) = class_init
                .body_statements
                .iter()
                .flat_map(Statement::iter_declarations)
                .find(|decl| decl.name == var_name && decl.type_info.base_type == "GParamSpec")
            {
                fixes.push(Fix::delete_line(&decl.location, source));
            }
        }

        // Add g_object_class_install_properties call after all assignments
        if let Some(last_call) = install_calls.last() {
            let Some(last_stmt) = self.find_statement_containing_call(
                &class_init.body_statements,
                last_call.location.start_byte,
            ) else {
                return fixes;
            };

            let call = style.format_call_stmt(
                "g_object_class_install_properties",
                &[object_class_var, &n_props_name, &array_name],
            );
            let install_properties_call = format!("\n\n{}{}", indentation, call);
            let last_stmt_end = last_stmt.location().find_semicolon_end(source);
            fixes.push(Fix::new(
                last_stmt_end,
                last_stmt_end,
                install_properties_call,
            ));
        }

        fixes
    }

    /// Determine the N_PROPS name based on enum naming convention
    fn determine_n_props_name(&self, property_enum: &EnumInfo) -> String {
        // Look for common prefixes in enum values
        if let Some(first_value) = property_enum.values.first() {
            let name = &first_value.name;

            // Check for common patterns like PROP_0, WIDGET_PROP_0, etc.
            if let Some(prefix_end) = name.rfind("PROP_") {
                let prefix = &name[..prefix_end];
                if prefix.is_empty() {
                    return "N_PROPS".to_string();
                } else {
                    return format!("{}N_PROPS", prefix);
                }
            }
        }

        "N_PROPS".to_string()
    }

    /// Determine the array name, preferring "props" but using "obj_props" if
    /// "props" exists
    fn determine_array_name(&self, file: &FileModel, _source: &[u8]) -> String {
        // Check if "props" is already used as a GParamSpec array
        for item in &file.top_level_items {
            if let TopLevelItem::Declaration(decl) = item
                && decl.name == "props"
                && decl.type_info.is_base_type("GParamSpec")
            {
                return "obj_props".to_string();
            }
        }

        "props".to_string()
    }

    fn find_statement_containing_call<'a>(
        &self,
        statements: &'a [Statement],
        call_start_byte: usize,
    ) -> Option<&'a Statement> {
        for stmt in statements {
            let loc = stmt.location();
            if call_start_byte >= loc.start_byte && call_start_byte < loc.end_byte {
                return Some(stmt);
            }
        }
        None
    }

    /// Re-indent multiline text to align continuation lines to a specific
    /// column
    fn reindent_multiline(&self, text: &str, target_column: usize) -> String {
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() <= 1 {
            return text.to_string();
        }

        let continuation_indent = " ".repeat(target_column);

        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                result.push_str(line);
            } else {
                result.push('\n');
                result.push_str(&continuation_indent);
                result.push_str(line.trim_start());
            }
        }

        result
    }
}
