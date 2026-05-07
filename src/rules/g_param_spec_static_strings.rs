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

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
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

        for (path, file) in ast_context.iter_c_files() {
            for func in file.iter_function_definitions() {
                for assignment in func.find_param_spec_assignments(&file.source) {
                    let Some(call) = assignment.param_spec_call() else {
                        continue;
                    };
                    self.check_call(path, call, assignment.property(), &static_flags, violations);
                }
            }
        }
    }
}

impl GParamSpecStaticStrings {
    fn check_call(
        &self,
        file_path: &std::path::Path,
        call: &CallExpression,
        property: &Property,
        custom_static_flags: &[String],
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() < 4 {
            return;
        }

        let nick_is_literal = property.nick.is_some();
        let blurb_is_literal = property.blurb.is_some();

        let has_static_strings = property.flags.contains(&ParamFlag::StaticStrings);
        let has_static_name = property.flags.contains(&ParamFlag::StaticName);
        let has_static_nick = property.flags.contains(&ParamFlag::StaticNick);
        let has_static_blurb = property.flags.contains(&ParamFlag::StaticBlurb);

        let has_custom_static_flag = property.flags.iter().any(|flag| {
            if let ParamFlag::Unknown(name) = flag {
                custom_static_flags.contains(name)
            } else {
                false
            }
        });

        let is_satisfied = if has_static_strings || has_custom_static_flag {
            true
        } else if nick_is_literal && blurb_is_literal {
            has_static_name && has_static_nick && has_static_blurb
        } else if nick_is_literal {
            has_static_name && has_static_nick
        } else if blurb_is_literal {
            has_static_name && has_static_blurb
        } else {
            has_static_name
        };

        if is_satisfied {
            return;
        }

        let needed = self.needed_flags(nick_is_literal, blurb_is_literal);
        let new_flags = self.build_fixed_flags(&property.flags, &needed);
        let needed_desc = needed
            .iter()
            .map(gobject_ast::ParamFlag::as_str)
            .collect::<Vec<_>>()
            .join(" | ");

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

    fn needed_flags(&self, nick_is_literal: bool, blurb_is_literal: bool) -> Vec<ParamFlag> {
        match (nick_is_literal, blurb_is_literal) {
            (true, true) => vec![ParamFlag::StaticStrings],
            (true, false) => vec![ParamFlag::StaticName, ParamFlag::StaticNick],
            (false, true) => vec![ParamFlag::StaticName, ParamFlag::StaticBlurb],
            (false, false) => vec![ParamFlag::StaticName],
        }
    }

    fn build_fixed_flags(&self, current_flags: &[ParamFlag], needed_flags: &[ParamFlag]) -> String {
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
