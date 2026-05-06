use std::{path::Path, sync::LazyLock};

use gobject_ast::{
    TypeInfo,
    model::top_level::{StructField, TopLevelItem, TypeDefItem, TypedefTarget},
    types::Parameter,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{ConfigOption, Fix, Rule, Violation},
};

pub struct TypeStyle;

impl Rule for TypeStyle {
    fn name(&self) -> &'static str {
        "type_style"
    }

    fn description(&self) -> &'static str {
        "Enforce consistent use of GLib or C standard type aliases"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn config_options(&self) -> &'static [ConfigOption] {
        static OPTIONS: LazyLock<Vec<ConfigOption>> = LazyLock::new(|| {
            vec![ConfigOption {
                name: "style",
                option_type: "string",
                default_value: "\"glib\"",
                example_value: "\"c\"",
                description: "Type style to enforce: \"glib\" (prefer gint, gchar, …) or \"c\" (prefer int, char, int32_t, …)",
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
        let style = config
            .get_rule_config(self.name())
            .and_then(|rc| rc.options.get("style"))
            .and_then(|v| v.as_str())
            .unwrap_or("glib");

        for (path, file) in ast_context.iter_all_files() {
            for item in file.iter_all_items() {
                self.check_item(item, path, style, violations);
            }
        }
    }
}

impl TypeStyle {
    fn check_item(
        &self,
        item: &TopLevelItem,
        path: &Path,
        style: &str,
        violations: &mut Vec<Violation>,
    ) {
        match item {
            TopLevelItem::FunctionDeclaration(decl) => {
                self.check_type(&decl.return_type, path, style, violations);
                self.check_params(&decl.parameters, path, style, violations);
            }
            TopLevelItem::FunctionDefinition(def) => {
                self.check_type(&def.return_type, path, style, violations);
                self.check_params(&def.parameters, path, style, violations);
                for var in def.iter_local_declarations() {
                    self.check_type(&var.type_info, path, style, violations);
                }
            }
            TopLevelItem::TypeDefinition(typedef_item) => {
                self.check_typedef(typedef_item, path, style, violations);
            }
            TopLevelItem::Declaration(decl) => {
                self.check_type(&decl.type_info, path, style, violations);
            }
            _ => {}
        }
    }

    fn check_typedef(
        &self,
        item: &TypeDefItem,
        path: &Path,
        style: &str,
        violations: &mut Vec<Violation>,
    ) {
        match item {
            TypeDefItem::Typedef {
                target,
                struct_fields,
                ..
            } => {
                match target {
                    TypedefTarget::Type(type_info) => {
                        self.check_type(type_info, path, style, violations);
                    }
                    TypedefTarget::Callback {
                        return_type,
                        parameters,
                    } => {
                        self.check_type(return_type, path, style, violations);
                        self.check_params(parameters, path, style, violations);
                    }
                }
                self.check_fields(struct_fields, path, style, violations);
            }
            TypeDefItem::Struct { fields, .. } => {
                self.check_fields(fields, path, style, violations);
            }
            TypeDefItem::Enum(_) => {}
        }
    }

    fn check_fields(
        &self,
        fields: &[StructField],
        path: &Path,
        style: &str,
        violations: &mut Vec<Violation>,
    ) {
        for field in fields {
            field.walk(&mut |f| {
                self.check_type(&f.field_type, path, style, violations);
            });
        }
    }

    fn check_params(
        &self,
        parameters: &[Parameter],
        path: &Path,
        style: &str,
        violations: &mut Vec<Violation>,
    ) {
        for param in parameters {
            if let Parameter::Regular { type_info, .. } = param {
                self.check_type(type_info, path, style, violations);
            }
        }
    }

    fn check_type(
        &self,
        type_info: &TypeInfo,
        path: &Path,
        style: &str,
        violations: &mut Vec<Violation>,
    ) {
        let Some(basic) = type_info.as_basic() else {
            return;
        };

        let canonical = if style == "c" {
            basic.canonical_c(&type_info.base_type)
        } else {
            basic.canonical_glib()
        };

        let Some(canonical) = canonical else { return };

        if type_info.base_type == canonical {
            return;
        }

        let new_text = type_info
            .full_text
            .replacen(&type_info.base_type, canonical, 1);
        let fix = Fix::new(
            type_info.location.start_byte,
            type_info.location.end_byte,
            new_text,
        );

        violations.push(self.violation_with_fix(
            path,
            type_info.location.line,
            type_info.location.column,
            format!("use `{}` instead of `{}`", canonical, type_info.base_type),
            fix,
        ));
    }
}
