use gobject_ast::model::{
    CallExpression, EnumInfo, Expression, FileModel, FunctionDefItem, GType, ParamSpecAssignment,
    Statement,
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
        ast_context: &AstContext,
        config: &Config,
        enum_info: &EnumInfo,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if !enum_info.is_property_enum() {
            return;
        }

        let Some(mut ctx) = file.resolve_property_enum_context(enum_info) else {
            return;
        };
        // Prefer the Define variant which has interfaces populated
        if ctx.gobject_type.interfaces.is_empty()
            && let Some(define) = file
                .iter_all_gobject_types()
                .find(|gt| gt.type_name == ctx.gobject_type.type_name && gt.kind.is_define())
        {
            ctx.gobject_type = define;
        }

        let individual_properties: Vec<_> = ctx
            .gobject_type
            .properties
            .iter()
            .filter(|p| !matches!(p, ParamSpecAssignment::ArraySubscript { .. }))
            .collect();
        if individual_properties.is_empty() {
            return;
        }

        let fixes = self.generate_fixes(
            ast_context,
            file,
            ctx.class_init,
            ctx.gobject_type,
            &individual_properties,
            enum_info,
            &config.style,
        );

        let location = individual_properties
            .first()
            .map(|p| p.statement_location())
            .unwrap();
        let message = if fixes.is_empty() {
            format!(
                "Consider using g_object_class_install_properties() instead of {} individual property installation calls",
                individual_properties.len()
            )
        } else {
            format!(
                "Use g_object_class_install_properties() instead of {} individual property installation calls",
                individual_properties.len()
            )
        };

        violations.push(self.violation_with_fixes_at(&file.path, location, message, fixes));
    }
}

impl UseGObjectClassInstallProperties {
    #[allow(clippy::too_many_arguments)]
    fn generate_fixes(
        &self,
        ast_context: &AstContext,
        file: &FileModel,
        class_init: &FunctionDefItem,
        gobject_type: &gobject_ast::model::GObjectType,
        assignments: &[&ParamSpecAssignment],
        property_enum: &EnumInfo,
        style: &crate::config::Style,
    ) -> Vec<Fix> {
        let mut fixes = Vec::new();

        let install_calls: Vec<&CallExpression> = assignments
            .iter()
            .filter_map(|a| match a {
                ParamSpecAssignment::DirectInstall { install_call, .. } => Some(install_call),
                ParamSpecAssignment::Variable { install_call, .. } => install_call.as_ref(),
                _ => None,
            })
            .collect();
        let override_calls: Vec<&CallExpression> = assignments
            .iter()
            .filter_map(|a| match a {
                ParamSpecAssignment::OverrideProperty { call, .. } => Some(call),
                _ => None,
            })
            .collect();

        // Check which override properties can be resolved to an interface
        let has_convertible_overrides = !override_calls.is_empty()
            && override_calls.iter().any(|call| {
                call.get_arg(2)
                    .and_then(|a| a.location().as_str())
                    .map(|s| s.trim_matches('"'))
                    .and_then(|name| {
                        ast_context
                            .project
                            .find_interface_for_property(gobject_type, name)
                    })
                    .is_some()
            });
        if install_calls.is_empty() && !has_convertible_overrides {
            return fixes;
        }

        let multiple_types = file
            .iter_all_gobject_types()
            .filter(|gt| gt.type_name != gobject_type.type_name)
            .any(|gt| !gt.properties.is_empty());

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

        // Handle split enum: collapse the intermediate sentinel only when we
        // can convert overrides to g_param_spec_override (i.e. interface found)
        let split_sentinel = self.find_split_sentinel(property_enum);
        let mut deleted_sentinel_name: Option<&str> = None;
        if has_convertible_overrides
            && let Some((sentinel_value, first_override_value)) = &split_sentinel
        {
            if first_override_value.is_prop_last() {
                // Pattern: PROP_X, N_PROPS = PROP_X — just remove the
                // initializer so N_PROPS auto-increments past PROP_X
                if let Some(value_loc) = &first_override_value.value_location {
                    let eq_start = value_loc.find_before(b'=');
                    fixes.push(Fix::new(eq_start, value_loc.end_byte, String::new()));
                }
            } else {
                // True split: NUM_PROPERTIES, PROP_OVERRIDE = NUM_PROPERTIES —
                // delete the intermediate sentinel and remove the initializer
                fixes.push(Fix::delete_line_and_trailing_blank(
                    &sentinel_value.location,
                ));
                deleted_sentinel_name = Some(&sentinel_value.name);
                if let Some(value_loc) = &first_override_value.value_location {
                    let eq_start = value_loc.find_before(b'=');
                    fixes.push(Fix::new(eq_start, value_loc.end_byte, String::new()));
                }
            }
        }

        // Check if enum has N_PROPS (the final sentinel), skipping any
        // sentinel we're about to delete from a split enum collapse
        let n_props_value =
            property_enum.values.iter().rev().find(|v| {
                v.is_prop_last() && deleted_sentinel_name.is_none_or(|name| v.name != name)
            });

        // If override properties appear after N_PROPS in the enum, it must
        // move to the end so the array is large enough
        let n_props_mispositioned = has_convertible_overrides
            && n_props_value.is_some_and(|nv| {
                let nv_idx = property_enum
                    .values
                    .iter()
                    .position(|v| v.name == nv.name)
                    .unwrap();
                override_calls.iter().any(|call| {
                    call.get_arg(1)
                        .and_then(|a| a.location().as_str())
                        .and_then(|name| property_enum.values.iter().position(|v| v.name == name))
                        .is_some_and(|idx| idx > nv_idx)
                })
            });

        if n_props_mispositioned
            && let Some(nv) = n_props_value {
                fixes.push(Fix::delete_line_and_trailing_blank(&nv.location));
            }

        let n_props_name = if let Some(n_props) = n_props_value
            && !n_props_mispositioned
        {
            n_props.name.clone()
        } else {
            let n_props_name = n_props_value
                .map(|v| v.name.clone())
                .or_else(|| deleted_sentinel_name.map(std::string::ToString::to_string))
                .unwrap_or_else(|| {
                    let name = self.determine_n_props_name(property_enum);
                    if multiple_types && name == "N_PROPS" {
                        format!("{}_N_PROPS", gobject_type.function_prefix.to_uppercase())
                    } else {
                        name
                    }
                });

            // Insert N_PROPS after the last enum value
            let last_value = property_enum.values.last().unwrap();
            let value_indentation = last_value.location.extract_indentation();

            let comma_end = last_value.location.find_after(b',');
            let (insertion_pos, needs_comma) = if comma_end > last_value.location.end_byte {
                (comma_end, false)
            } else {
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

        // Find existing array from ArraySubscript assignments, or from typed arrays
        let enum_member_names: Vec<&str> = property_enum
            .values
            .iter()
            .map(|v| v.name.as_str())
            .collect();
        let existing_array_name = assignments
            .iter()
            .find_map(|a| {
                if let ParamSpecAssignment::ArraySubscript { array_name, .. } = a {
                    Some(array_name.as_str())
                } else {
                    None
                }
            })
            .or_else(|| {
                file.find_typed_arrays("GParamSpec", true, None)
                    .into_iter()
                    .find(|d| {
                        matches!(&d.array_size, Some(Expression::Identifier(id)) if enum_member_names.contains(&id.name.as_str()))
                    })
                    .map(|d| d.name.as_str())
            });

        let array_name = if let Some(name) = existing_array_name {
            name.to_string()
        } else {
            if multiple_types {
                format!("{}_props", gobject_type.function_prefix)
            } else {
                "props".to_string()
            }
        };

        if existing_array_name.is_some() {
            // Find the declaration and update its size to N_PROPS
            if let Some(decl) = file
                .find_typed_arrays("GParamSpec", true, None)
                .into_iter()
                .find(|d| d.name == array_name)
                && let Some(Expression::Identifier(size_id)) = &decl.array_size
                && size_id.name != n_props_name
            {
                fixes.push(Fix::new(
                    size_id.location.start_byte,
                    size_id.location.end_byte,
                    n_props_name.clone(),
                ));
            }
        } else {
            // Add GParamSpec array declaration after the enum
            let semicolon_end = property_enum.location.find_after(b';');
            let insertion_pos = if semicolon_end > property_enum.location.end_byte {
                semicolon_end
            } else {
                property_enum.location.end_byte
            };

            let array_decl = format!(
                "\n\nstatic GParamSpec *{}[{}] = {{ NULL, }};",
                array_name, n_props_name
            );
            fixes.push(Fix::new(insertion_pos, insertion_pos, array_decl));
        }

        // Find the GObjectClass declaration to get object_class variable name and
        // indentation
        let object_class_var = class_init
            .iter_local_declarations()
            .find(|decl| decl.type_info.base_type == "GObjectClass")
            .map_or("object_class", |decl| decl.name.as_str());

        // Get indentation for the install_properties call
        let all_calls_for_indent = install_calls.first().or(override_calls.first()).copied();
        let indentation = if let Some(first_call) = all_calls_for_indent {
            if let Some(stmt) = self.find_statement_containing_call(
                &class_init.body_statements,
                first_call.location.start_byte,
            ) {
                stmt.location().extract_indentation()
            } else {
                "  ".to_string()
            }
        } else {
            "  ".to_string()
        };

        // Track GParamSpec variable names to delete their declarations later
        let mut param_spec_vars = std::collections::HashSet::new();

        // Convert each g_object_class_install_property call
        for call in &install_calls {
            // Extract the property enum value (2nd argument)
            let Some(prop_id_arg) = call.get_arg(1) else {
                continue;
            };
            let Some(prop_id) = prop_id_arg.location().as_str() else {
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
                let func_name = param_spec_call.function_name();
                let new_line_prefix = format!("{}[{}] = {} (", array_name, prop_id, func_name);
                let target_column = indentation.len() + new_line_prefix.len();

                let Some(param_spec_text) = param_spec_arg.location().as_str() else {
                    continue;
                };
                (
                    self.reindent_multiline(param_spec_text, target_column),
                    false,
                )
            } else {
                // Variable pattern: param_spec = g_param_spec_xxx(...);
                // g_object_class_install_property(..., param_spec);
                let Some(var_name) = param_spec_arg.location().as_str() else {
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
                    let func_name = g_param_spec_call.function_name();
                    let new_line_prefix = format!("{}[{}] = {} (", array_name, prop_id, func_name);
                    // Note: indentation is not included because it stays in place during
                    // replacement
                    let assignment_indent = statement_location.extract_indentation();
                    let target_column = assignment_indent.len() + new_line_prefix.len();

                    let Some(param_spec_text) = g_param_spec_call.location.as_str() else {
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
                        statement_location.find_semicolon_end(),
                        replacement,
                    ));

                    (String::new(), true) // Mark to delete install_property call
                } else {
                    // Fallback - just use the variable name as-is
                    let Some(param_spec_text) = param_spec_arg.location().as_str() else {
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
                fixes.push(Fix::delete_line(stmt.location()));
            } else {
                // Replace the statement with array assignment
                let replacement = format!("{}[{}] = {};", array_name, prop_id, param_spec);
                fixes.push(Fix::new(
                    stmt.location().start_byte,
                    stmt.location().find_semicolon_end(),
                    replacement,
                ));
            }
        }

        // Convert each g_object_class_override_property call
        for call in &override_calls {
            let Some(prop_id_arg) = call.get_arg(1) else {
                continue;
            };
            let Some(prop_id) = prop_id_arg.location().as_str() else {
                continue;
            };

            let Some(prop_name_arg) = call.get_arg(2) else {
                continue;
            };
            let Some(prop_name) = prop_name_arg.location().as_str() else {
                continue;
            };
            let prop_name = prop_name.trim_matches('"');

            let Some(stmt) = self.find_statement_containing_call(
                &class_init.body_statements,
                call.location.start_byte,
            ) else {
                continue;
            };

            let resolved = ast_context
                .project
                .find_interface_for_property(gobject_type, prop_name);
            if let Some(iface_gtype) = resolved {
                let GType::Identifier(iface_type_str) = iface_gtype else {
                    continue;
                };

                let iface_ref =
                    style.format_call("g_type_default_interface_ref", &[iface_type_str]);
                let prop_name_quoted = format!("\"{}\"", prop_name);
                let find_prop = style.format_call(
                    "g_object_interface_find_property",
                    &[&iface_ref, &prop_name_quoted],
                );
                let replacement = format!(
                    "{}[{}] = g_param_spec_override (\"{}\",\n{}    {});",
                    array_name, prop_id, prop_name, indentation, find_prop
                );
                fixes.push(Fix::new(
                    stmt.location().start_byte,
                    stmt.location().find_semicolon_end(),
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
                fixes.push(Fix::delete_line(&decl.location));
            }
        }

        // Ensure g_object_class_install_properties exists after all property
        // assignments. Only consider override calls when we converted them.
        let last_call = if has_convertible_overrides {
            [
                install_calls.last().copied(),
                override_calls.last().copied(),
            ]
            .into_iter()
            .flatten()
            .max_by_key(|c| c.location.start_byte)
        } else {
            install_calls.last().copied()
        };
        if let Some(last_call) = last_call {
            let Some(last_stmt) = self.find_statement_containing_call(
                &class_init.body_statements,
                last_call.location.start_byte,
            ) else {
                return fixes;
            };
            let last_stmt_end = last_stmt.location().find_semicolon_end();

            let existing_install = class_init
                .find_calls_matching(|name| name == "g_object_class_install_properties")
                .into_iter()
                .next();

            if let Some(existing) = existing_install {
                if existing.location.start_byte < last_call.location.start_byte {
                    let Some(existing_stmt) = self.find_statement_containing_call(
                        &class_init.body_statements,
                        existing.location.start_byte,
                    ) else {
                        return fixes;
                    };
                    fixes.push(Fix::delete_line_and_trailing_blank(
                        existing_stmt.location(),
                    ));

                    let call = style.format_call_stmt(
                        "g_object_class_install_properties",
                        &[object_class_var, &n_props_name, &array_name],
                    );
                    let install_properties_call = format!("\n\n{}{}", indentation, call);
                    fixes.push(Fix::new(
                        last_stmt_end,
                        last_stmt_end,
                        install_properties_call,
                    ));
                }
            } else {
                let call = style.format_call_stmt(
                    "g_object_class_install_properties",
                    &[object_class_var, &n_props_name, &array_name],
                );
                let install_properties_call = format!("\n\n{}{}", indentation, call);
                fixes.push(Fix::new(
                    last_stmt_end,
                    last_stmt_end,
                    install_properties_call,
                ));
            }
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

    /// Find a split enum sentinel: an intermediate sentinel like N_REAL_PROPS
    /// followed by an enum value with `= N_REAL_PROPS` initializer.
    fn find_split_sentinel<'a>(
        &self,
        property_enum: &'a EnumInfo,
    ) -> Option<(
        &'a gobject_ast::model::EnumValue,
        &'a gobject_ast::model::EnumValue,
    )> {
        for (i, value) in property_enum.values.iter().enumerate() {
            if let Some(Expression::Identifier(id)) = &value.value_expr
                && let Some(sentinel) = property_enum.values[..i].iter().find(|v| v.name == id.name)
            {
                return Some((sentinel, value));
            }
        }
        None
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
