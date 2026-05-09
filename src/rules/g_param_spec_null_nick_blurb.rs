use gobject_ast::model::{Argument, CallExpression, FileModel, GObjectType, ParamFlag, Property};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct GParamSpecNullNickBlurb;

impl Rule for GParamSpecNullNickBlurb {
    fn name(&self) -> &'static str {
        "g_param_spec_null_nick_blurb"
    }

    fn description(&self) -> &'static str {
        "Ensure g_param_spec_* functions have NULL for nick and blurb parameters"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Pedantic
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_gobject_type(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        gobject_type: &GObjectType,
        file: &FileModel,
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

        for assignment in &gobject_type.properties {
            let Some(call) = assignment.param_spec_call() else {
                continue;
            };
            self.check_call(file, call, assignment.property(), &static_flags, violations);
        }
    }
}

impl GParamSpecNullNickBlurb {
    fn check_call(
        &self,
        file: &FileModel,
        call: &CallExpression,
        property: &Property,
        custom_static_flags: &[String],
        violations: &mut Vec<Violation>,
    ) {
        if call.arguments.len() < 3 {
            return;
        }

        let has_custom_static_flag = property.flags.iter().any(|flag| {
            if let ParamFlag::Unknown(name) = flag {
                custom_static_flags.contains(name)
            } else {
                false
            }
        });

        if has_custom_static_flag {
            return;
        }

        let Some(nick_expr) = call.get_arg(1) else {
            return;
        };
        let Some(blurb_expr) = call.get_arg(2) else {
            return;
        };

        let nick_is_null = nick_expr.is_null();
        let blurb_is_null = blurb_expr.is_null();

        let mut issues = Vec::new();
        if !nick_is_null {
            issues.push("nick (parameter 2)");
        }
        if !blurb_is_null {
            issues.push("blurb (parameter 3)");
        }

        if issues.is_empty() {
            return;
        }

        let string_fix = if !nick_is_null && !blurb_is_null {
            Fix::new(
                nick_expr.location().start_byte,
                blurb_expr.location().end_byte,
                "NULL, NULL",
            )
        } else if !nick_is_null {
            Fix::new(
                nick_expr.location().start_byte,
                nick_expr.location().end_byte,
                "NULL",
            )
        } else {
            Fix::new(
                blurb_expr.location().start_byte,
                blurb_expr.location().end_byte,
                "NULL",
            )
        };

        let mut fixes = vec![string_fix];

        if let Some(new_flags) = self.compute_new_flags(&property.flags) {
            let Argument::Expression(flags_expr) = call.arguments.last().unwrap();
            fixes.push(Fix::new(
                flags_expr.location().start_byte,
                flags_expr.location().end_byte,
                new_flags,
            ));
        }

        violations.push(self.violation_with_fixes(
            &file.path,
            call.location.line,
            call.location.column,
            format!(
                "{} should have NULL for {}",
                call.function_name(&file.source),
                issues.join(" and ")
            ),
            fixes,
        ));
    }

    fn compute_new_flags(&self, current_flags: &[ParamFlag]) -> Option<String> {
        let needs_removal = current_flags.iter().any(|f| {
            matches!(
                f,
                ParamFlag::StaticNick | ParamFlag::StaticBlurb | ParamFlag::StaticStrings
            )
        });
        let has_name = current_flags
            .iter()
            .any(|f| matches!(f, ParamFlag::StaticName));

        if !needs_removal && has_name {
            return None;
        }

        let mut new_flags: Vec<ParamFlag> = current_flags
            .iter()
            .filter(|f| {
                !matches!(
                    f,
                    ParamFlag::StaticNick | ParamFlag::StaticBlurb | ParamFlag::StaticStrings
                )
            })
            .cloned()
            .collect();

        if !new_flags.iter().any(|f| matches!(f, ParamFlag::StaticName)) {
            new_flags.push(ParamFlag::StaticName);
        }

        Some(if new_flags.is_empty() {
            "0".to_string()
        } else {
            new_flags
                .iter()
                .map(ParamFlag::as_str)
                .collect::<Vec<_>>()
                .join(" | ")
        })
    }
}
