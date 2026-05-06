use std::sync::LazyLock;

use gobject_ast::{
    CallExpression,
    types::{ParamFlag, Property},
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{ConfigOption, Fix, Rule, Violation},
};

pub struct GParamSpecStaticStrings;

impl Rule for GParamSpecStaticStrings {
    fn name(&self) -> &'static str {
        "g_param_spec_static_strings"
    }

    fn description(&self) -> &'static str {
        "Ensure *_param_spec_* calls use G_PARAM_STATIC_STRINGS flag for string literals"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Perf
    }

    fn fixable(&self) -> bool {
        true
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: LazyLock<Vec<ConfigOption>> = LazyLock::new(|| {
            vec![ConfigOption {
                name: "static_flags",
                option_type: "array<string>",
                default_value: "[]",
                example_value: "[\"ST_PARAM_READWRITE\", \"ST_PARAM_READABLE\"]",
                description: "List of custom flag constants that already include G_PARAM_STATIC_STRINGS",
            }]
        });

        &OPTIONS
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        path: &std::path::Path,
        violations: &mut Vec<Violation>,
    ) {
        // Get custom flags that already include static strings
        let static_flags = config
            .get_rule_config(self.name())
            .and_then(|rc| rc.options.get("static_flags"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Find all *_param_spec_* calls (including g_param_spec_*, cogl_param_spec_*,
        // etc.) but skip *_param_spec_override and *_param_spec_internal
        for call in func.find_calls_matching(|name| {
            name.contains("_param_spec_")
                && !name.ends_with("_param_spec_override")
                && !name.ends_with("_param_spec_internal")
        }) {
            self.check_call(path, call, &static_flags, violations);
        }
    }
}

impl GParamSpecStaticStrings {
    fn check_call(
        &self,
        file_path: &std::path::Path,
        call: &CallExpression,
        custom_static_flags: &[String],
        violations: &mut Vec<Violation>,
    ) {
        // g_param_spec_*(name, nick, blurb, ..., flags) — need at least 4 args
        if call.arguments.len() < 4 {
            return;
        }

        // Parse the property using the AST helpers
        let Some(property) = Property::from_param_spec_call(call) else {
            return;
        };

        // If we successfully parsed the property, name is always a literal
        // Check if nick/blurb are literals or NULL using the Option<String>
        let nick_is_literal = property.nick.is_some();
        let blurb_is_literal = property.blurb.is_some();

        // Check what static flags are present using the typed ParamFlag enum
        let has_static_strings = property.flags.contains(&ParamFlag::StaticStrings);
        let has_static_name = property.flags.contains(&ParamFlag::StaticName);
        let has_static_nick = property.flags.contains(&ParamFlag::StaticNick);
        let has_static_blurb = property.flags.contains(&ParamFlag::StaticBlurb);

        // Check if any custom flags that include static strings are present
        let has_custom_static_flag = property.flags.iter().any(|flag| {
            if let ParamFlag::Unknown(name) = flag {
                custom_static_flags.contains(name)
            } else {
                false
            }
        });

        // Is the minimal required set of static flags already present?
        let is_satisfied = if has_static_strings || has_custom_static_flag {
            // G_PARAM_STATIC_STRINGS or custom flag covers everything — always satisfied
            true
        } else if nick_is_literal && blurb_is_literal {
            // All three strings are literals — need NAME + NICK + BLURB
            has_static_name && has_static_nick && has_static_blurb
        } else if nick_is_literal {
            has_static_name && has_static_nick
        } else if blurb_is_literal {
            has_static_name && has_static_blurb
        } else {
            // nick and blurb are NULL — only the name needs the static flag
            has_static_name
        };

        if is_satisfied {
            return;
        }

        // Build the fix using typed flags
        let needed = self.needed_flags(nick_is_literal, blurb_is_literal);
        let new_flags = self.build_fixed_flags(&property.flags, &needed);
        let needed_desc = needed
            .iter()
            .map(gobject_ast::ParamFlag::as_str)
            .collect::<Vec<_>>()
            .join(" | ");

        // Get the flags expression location for the fix
        let gobject_ast::Argument::Expression(flags_expr) = call.arguments.last().unwrap();
        let fix = Fix::new(
            flags_expr.location().start_byte,
            flags_expr.location().end_byte,
            new_flags,
        );

        violations.push(self.violation_with_fix(
            file_path,
            call.location.line,
            call.location.column,
            format!(
                "Add {} to {} flags (saves memory for static strings)",
                needed_desc,
                call.function_name()
            ),
            fix,
        ));
    }

    /// Return the flags that should be added, given which args are literals
    fn needed_flags(&self, nick_is_literal: bool, blurb_is_literal: bool) -> Vec<ParamFlag> {
        match (nick_is_literal, blurb_is_literal) {
            (true, true) => vec![ParamFlag::StaticStrings],
            (true, false) => vec![ParamFlag::StaticName, ParamFlag::StaticNick],
            (false, true) => vec![ParamFlag::StaticName, ParamFlag::StaticBlurb],
            (false, false) => vec![ParamFlag::StaticName],
        }
    }

    /// Build the replacement flags string: remove static flags and add the
    /// needed ones
    fn build_fixed_flags(&self, current_flags: &[ParamFlag], needed_flags: &[ParamFlag]) -> String {
        // Filter out static flags, keep everything else
        let mut new_flags: Vec<ParamFlag> = current_flags
            .iter()
            .filter(|f| {
                !matches!(
                    f,
                    ParamFlag::StaticName
                        | ParamFlag::StaticNick
                        | ParamFlag::StaticBlurb
                        | ParamFlag::StaticStrings
                )
            })
            .cloned()
            .collect();

        // Append the needed flags
        new_flags.extend_from_slice(needed_flags);

        if new_flags.is_empty() {
            "0".to_string()
        } else {
            new_flags
                .iter()
                .map(gobject_ast::ParamFlag::as_str)
                .collect::<Vec<_>>()
                .join(" | ")
        }
    }
}
