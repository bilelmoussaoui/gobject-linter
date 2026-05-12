use std::collections::HashMap;

use gobject_ast::model::{
    Expression, FileModel, FunctionDefItem, ParamFlag, ParamSpecAssignment, Property, PropertyType,
    Statement, SwitchStatement,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{ConfigOption, Fix, Rule, Violation},
};

pub struct PropertySwitchExhaustiveness;

impl Rule for PropertySwitchExhaustiveness {
    fn name(&self) -> &'static str {
        "property_switch_exhaustiveness"
    }

    fn description(&self) -> &'static str {
        "Ensure get_property/set_property switch statements handle all required properties"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Correctness
    }

    fn fixable(&self) -> bool {
        true
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        &[
            ConfigOption {
                name: "style",
                option_type: "string",
                default_value: "\"typed\"",
                example_value: "\"legacy\"",
                description: "Property enum style: \"typed\" (strict, requires enum casts and all properties in switches) or \"legacy\" (relaxed, only checks read-write properties)",
            },
            ConfigOption {
                name: "readable_flags",
                option_type: "array<string>",
                default_value: "[]",
                example_value: "[\"MY_LIB_READABLE\", \"MY_LIB_READWRITE\"]",
                description: "Additional flag names indicating readable properties (G_PARAM_READABLE and G_PARAM_READWRITE are always included)",
            },
            ConfigOption {
                name: "writable_flags",
                option_type: "array<string>",
                default_value: "[]",
                example_value: "[\"MY_LIB_WRITABLE\", \"MY_LIB_READWRITE\"]",
                description: "Additional flag names indicating writable properties (G_PARAM_WRITABLE and G_PARAM_READWRITE are always included)",
            },
        ]
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let rule_config = &config.rules.property_switch_exhaustiveness;

        // Get style configuration (default to "typed")
        let style = rule_config
            .options
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("typed");

        // Get custom flag patterns from config
        let readable_flags = self.get_flag_patterns(rule_config, "readable_flags");
        let writable_flags = self.get_flag_patterns(rule_config, "writable_flags");

        for (_path, file) in ast_context.iter_all_files() {
            for enum_info in file.iter_property_enums() {
                // Find the associated GObjectType
                let Some(gobject_type) = file.find_gobject_type_for_property_enum(enum_info) else {
                    continue;
                };

                // Get property names (excluding PROP_0 and N_PROPS)
                let property_names: Vec<&str> = enum_info
                    .values
                    .iter()
                    .filter(|v| !v.is_prop_0() && !v.is_prop_last())
                    .map(|v| v.name.as_str())
                    .collect();

                if property_names.is_empty() {
                    continue;
                }

                // Build a map from property enum names to their access permissions
                let property_access = self.build_property_access_map(
                    &gobject_type.properties,
                    &file.source,
                    &readable_flags,
                    &writable_flags,
                );

                // Find get_property and set_property function names from class_init
                let class_init = file
                    .iter_function_definitions()
                    .find(|f| f.name == gobject_type.class_init_function_name());
                let get_property_func =
                    class_init.and_then(|ci| self.find_assigned_function(ci, "get_property"));
                let set_property_func =
                    class_init.and_then(|ci| self.find_assigned_function(ci, "set_property"));

                // Check get_property function
                if let Some(func_name) = get_property_func {
                    self.check_property_function(
                        file,
                        func_name,
                        &property_names,
                        &property_access,
                        true, // is_getter
                        style,
                        &config.style,
                        violations,
                    );
                }

                // Check set_property function
                if let Some(func_name) = set_property_func {
                    self.check_property_function(
                        file,
                        func_name,
                        &property_names,
                        &property_access,
                        false, // is_getter
                        style,
                        &config.style,
                        violations,
                    );
                }
            }
        }
    }
}

impl PropertySwitchExhaustiveness {
    /// Get flag patterns from config, extending the defaults with any extra
    /// flags the user specified
    fn get_flag_patterns(
        &self,
        rule_config: &crate::config::RuleConfig,
        option_name: &str,
    ) -> Vec<ParamFlag> {
        let mut flags = if option_name == "readable_flags" {
            vec![ParamFlag::Readable, ParamFlag::ReadWrite]
        } else {
            vec![ParamFlag::Writable, ParamFlag::ReadWrite]
        };

        if let Some(extra) = rule_config
            .options
            .get(option_name)
            .and_then(|v| v.as_array())
        {
            for flag in extra
                .iter()
                .filter_map(|v| v.as_str().map(ParamFlag::from_identifier))
            {
                if !flags.contains(&flag) {
                    flags.push(flag);
                }
            }
        }

        flags
    }

    /// Build a map of property names to their access permissions (readable,
    /// writable) Override properties return None to indicate unknown access
    /// type
    fn build_property_access_map<'a>(
        &self,
        assignments: &'a [ParamSpecAssignment],
        source: &'a [u8],
        readable_flags: &[ParamFlag],
        writable_flags: &[ParamFlag],
    ) -> HashMap<&'a str, Option<(bool, bool)>> {
        let mut access_map = HashMap::new();

        for assignment in assignments {
            if let Some(enum_val) = assignment.get_installed_enum_value(source) {
                let access =
                    self.get_property_access(assignment.property(), readable_flags, writable_flags);
                access_map.insert(enum_val, access);
            }
        }

        access_map
    }

    /// Determine property access type from flags
    /// Returns None for override properties (unknown access type), otherwise
    /// Some((is_readable, is_writable))
    fn get_property_access(
        &self,
        property: &Property,
        readable_flags: &[ParamFlag],
        writable_flags: &[ParamFlag],
    ) -> Option<(bool, bool)> {
        // Override properties: we can't determine access type, so return None
        // The rule will still check that they appear in switch statements
        if matches!(property.property_type, PropertyType::Override) {
            return None;
        }

        let has_readable = property.flags.iter().any(|f| readable_flags.contains(f));
        let has_writable = property.flags.iter().any(|f| writable_flags.contains(f));

        Some((has_readable, has_writable))
    }

    /// Find function name assigned to object_class->field in class_init
    fn find_assigned_function<'a>(
        &self,
        class_init: &'a FunctionDefItem,
        field_name: &str,
    ) -> Option<&'a str> {
        class_init
            .body_statements
            .iter()
            .flat_map(Statement::iter_assignments)
            .find_map(|assignment| {
                if let Expression::FieldAccess(field_access) = &*assignment.lhs
                    && field_access.field == field_name
                    && let Expression::Identifier(id) = &*assignment.rhs
                {
                    Some(id.name.as_str())
                } else {
                    None
                }
            })
    }

    /// Check a property getter or setter function for exhaustiveness
    #[allow(clippy::too_many_arguments)]
    fn check_property_function(
        &self,
        file: &FileModel,
        func_name: &str,
        property_names: &[&str],
        property_access: &HashMap<&str, Option<(bool, bool)>>,
        is_getter: bool,
        style: &str,
        call_style: &crate::config::Style,
        violations: &mut Vec<Violation>,
    ) {
        // Find the function definition
        let func = match file
            .iter_function_definitions()
            .find(|f| f.name == func_name)
        {
            Some(f) => f,
            None => return,
        };

        // Find top-level switch statements in the function body.
        // Only direct children — nested switches (e.g. inside delegation
        // `if` blocks) must not be checked for exhaustiveness.
        for stmt in &func.body_statements {
            let switch_stmt = match stmt {
                Statement::Switch(sw) => sw,
                _ => continue,
            };

            // Check if this switch is on prop_id or similar
            if !self.is_property_switch(&switch_stmt.condition) {
                continue;
            }

            let handled_cases = switch_stmt.case_identifiers();

            let mut missing_properties = Vec::new();

            for prop_name in property_names {
                if handled_cases.contains(prop_name) {
                    continue;
                }

                if style == "legacy" {
                    let access = property_access.get(*prop_name).copied().flatten();
                    if let Some((is_readable, is_writable)) = access {
                        if is_getter && !is_readable && is_writable {
                            continue;
                        }
                        if !is_getter && is_readable && !is_writable {
                            continue;
                        }
                    }
                }

                missing_properties.push(*prop_name);
            }

            let mut auto_fixable_properties = Vec::new();
            let mut has_non_fixable = false;

            for prop_name in &missing_properties {
                let access = property_access.get(*prop_name).copied().flatten();

                match access {
                    Some((is_readable, is_writable)) => {
                        let should_use_assert = if style == "typed" {
                            if is_getter {
                                !is_readable && is_writable
                            } else {
                                is_readable && !is_writable
                            }
                        } else {
                            false
                        };

                        if should_use_assert {
                            auto_fixable_properties.push(*prop_name);
                        } else {
                            has_non_fixable = true;
                            let message = format!(
                                "Property '{}' should be handled in {} switch statement",
                                prop_name, func_name
                            );
                            violations.push(self.violation(
                                &file.path,
                                switch_stmt.location.line,
                                1,
                                message,
                            ));
                        }
                    }
                    None => {
                        has_non_fixable = true;
                        let message = format!(
                            "Property '{}' should be handled in {} switch statement",
                            prop_name, func_name
                        );
                        violations.push(self.violation(
                            &file.path,
                            switch_stmt.location.line,
                            1,
                            message,
                        ));
                    }
                }
            }

            if !auto_fixable_properties.is_empty() {
                let can_remove_default = style == "typed"
                    && matches!(switch_stmt.condition, Expression::Cast(_))
                    && switch_stmt.has_default_case()
                    && !has_non_fixable;

                let fix = if can_remove_default {
                    self.generate_replace_default_with_cases_fix(
                        &auto_fixable_properties,
                        switch_stmt,
                        &file.source,
                        call_style,
                    )
                } else {
                    self.generate_insert_cases_fix(
                        &auto_fixable_properties,
                        switch_stmt,
                        &file.source,
                        call_style,
                    )
                };

                let message = if auto_fixable_properties.len() == 1 {
                    format!(
                        "Property '{}' should be handled in {} switch statement",
                        auto_fixable_properties[0], func_name
                    )
                } else {
                    format!(
                        "{} properties should be handled in {} switch statement",
                        auto_fixable_properties.len(),
                        func_name
                    )
                };
                violations.push(self.violation_with_fixes(
                    &file.path,
                    switch_stmt.location.line,
                    1,
                    message,
                    vec![fix],
                ));
            }

            if style == "typed"
                && missing_properties.is_empty()
                && matches!(switch_stmt.condition, Expression::Cast(_))
                && switch_stmt.has_default_case()
            {
                let (start, end) = self.find_default_case_range(switch_stmt, &file.source);
                let fix = Fix::delete(start, end);
                violations.push(self.violation_with_fixes(
                    &file.path,
                    switch_stmt.location.line,
                    1,
                    "Switch is exhaustive with enum cast; default case can be removed for compile-time checking".to_string(),
                    vec![fix],
                ));
            }
        }
    }

    /// Check if a switch condition is on a property ID variable
    fn is_property_switch(&self, condition: &Expression) -> bool {
        match condition {
            // Direct: switch (prop_id)
            Expression::Identifier(id) => {
                id.name == "prop_id" || id.name == "property_id" || id.name.ends_with("_prop_id")
            }
            // Cast: switch ((MyEnum) prop_id)
            Expression::Cast(cast) => self.is_property_switch(&cast.operand),
            _ => false,
        }
    }

    /// Generate a fix to insert multiple cases before default (without removing
    /// default)
    fn generate_insert_cases_fix(
        &self,
        prop_names: &[&str],
        switch_stmt: &SwitchStatement,
        source: &[u8],
        call_style: &crate::config::Style,
    ) -> Fix {
        let insertion_point = self.find_case_insertion_point(switch_stmt, source);
        let (case_indent, body_indent) = self.detect_indentation(switch_stmt, source);
        let assert_call = call_style.format_call_stmt("g_assert_not_reached", &[]);

        let mut replacement = String::new();
        for (i, prop_name) in prop_names.iter().enumerate() {
            if i + 1 < prop_names.len() {
                replacement.push_str(&format!("{}case {}:\n", case_indent, prop_name));
            } else {
                replacement.push_str(&format!(
                    "{}case {}:\n{}{}\n{}break;\n",
                    case_indent, prop_name, body_indent, assert_call, body_indent
                ));
            }
        }

        Fix::new(insertion_point, insertion_point, replacement)
    }

    /// Generate a fix to replace default case with new cases (combined
    /// operation)
    fn generate_replace_default_with_cases_fix(
        &self,
        prop_names: &[&str],
        switch_stmt: &SwitchStatement,
        source: &[u8],
        call_style: &crate::config::Style,
    ) -> Fix {
        let (case_indent, body_indent) = self.detect_indentation(switch_stmt, source);
        let assert_call = call_style.format_call_stmt("g_assert_not_reached", &[]);

        // Find the range of the default case to replace
        let (start, end) = self.find_default_case_range(switch_stmt, source);

        let mut replacement = String::new();
        for (i, prop_name) in prop_names.iter().enumerate() {
            if i + 1 < prop_names.len() {
                replacement.push_str(&format!("{}case {}:\n", case_indent, prop_name));
            } else {
                replacement.push_str(&format!(
                    "{}case {}:\n{}{}\n{}break;\n",
                    case_indent, prop_name, body_indent, assert_call, body_indent
                ));
            }
        }

        Fix::new(start, end, replacement)
    }

    /// Find the range to delete for the default case (used when replacing it)
    fn find_default_case_range(
        &self,
        switch_stmt: &SwitchStatement,
        source: &[u8],
    ) -> (usize, usize) {
        let default_case = switch_stmt
            .cases
            .iter()
            .find(|c| c.label.value.is_none())
            .unwrap();

        // Start from line beginning
        let (line_start, _) = default_case.label.location.find_line_bounds(source);

        // Find the last statement in the default case body
        let end_location = if let Some(last_stmt) = default_case.body.last() {
            *last_stmt.location()
        } else {
            // No statements in default case body, just use the case label location
            default_case.label.location
        };

        // Use the helper to get line bounds with following blank
        let (_, line_end) = end_location.find_line_bounds_with_following_blank(source);

        (line_start, line_end)
    }

    /// Find the byte position where a new case should be inserted
    fn find_case_insertion_point(&self, switch_stmt: &SwitchStatement, source: &[u8]) -> usize {
        // If there's a default case, insert before it
        if let Some(default_case) = switch_stmt.default_case() {
            let (line_start, _) = default_case.label.location.find_line_bounds(source);
            return line_start;
        }

        // Otherwise, find the last non-default case and insert after it
        let last_case = switch_stmt.cases.iter().rfind(|c| c.label.value.is_some());

        if let Some(last_case) = last_case {
            // Insert after the last statement in the case
            if let Some(last_stmt) = last_case.body.last() {
                let (_, line_end) = last_stmt
                    .location()
                    .find_line_bounds_with_following_blank(source);
                line_end
            } else {
                // No statements in case, insert right after the case label
                let (_, line_end) = last_case.label.location.find_line_bounds(source);
                line_end
            }
        } else {
            // No cases at all - insert at the beginning of the switch body
            switch_stmt.location.start_byte
        }
    }

    /// Detect indentation levels from existing cases
    /// Returns (case_indent, body_indent) where body_indent is for statements
    /// inside the case
    fn detect_indentation(&self, switch_stmt: &SwitchStatement, source: &[u8]) -> (String, String) {
        // Try to find a case with at least one statement in its body
        for case in &switch_stmt.cases {
            if let Some(first_stmt) = case.body.first() {
                let case_indent = case.label.location.extract_indentation(source);
                let body_indent = first_stmt.location().extract_indentation(source);
                return (case_indent, body_indent);
            }
        }

        // If no case has body statements, try to get case label indentation
        if let Some(case) = switch_stmt.cases.first() {
            let case_indent = case.label.location.extract_indentation(source);
            // Assume body is indented 2 more spaces than case
            let body_indent = format!("{}  ", case_indent);
            return (case_indent, body_indent);
        }

        // Default: case at 2 spaces, body at 4 spaces
        ("  ".to_string(), "    ".to_string())
    }
}
