use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGGnucFlagEnum;

impl Rule for UseGGnucFlagEnum {
    fn name(&self) -> &'static str {
        "use_g_gnuc_flag_enum"
    }

    fn description(&self) -> &'static str {
        "Use G_GNUC_FLAG_ENUM for enums that represent bit flags"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_enum(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        enum_info: &gobject_ast::EnumInfo,
        file: &gobject_ast::FileModel,
        path: &std::path::Path,
        violations: &mut Vec<Violation>,
    ) {
        let Some(ref enum_name) = enum_info.name else {
            return;
        };

        if !enum_info.is_flags_enum() {
            return;
        }

        if enum_info.has_attribute("G_GNUC_FLAG_ENUM") {
            return;
        }

        let source = &file.source;
        let fix = self.generate_fix(enum_info, source, enum_name);

        violations.push(self.violation_with_fix(
            path,
            enum_info.location.line,
            enum_info.location.column,
            format!(
                "Enum '{}' appears to be a flags enum but is missing G_GNUC_FLAG_ENUM attribute",
                enum_name
            ),
            fix,
        ));
    }
}

impl UseGGnucFlagEnum {
    fn generate_fix(
        &self,
        enum_info: &gobject_ast::types::EnumInfo,
        source: &[u8],
        enum_name: &str,
    ) -> Fix {
        let typedef_text = enum_info.location.as_str(source).unwrap_or("");

        if let Some(closing_brace_pos) = typedef_text.rfind('}') {
            let after_brace = &typedef_text[closing_brace_pos + 1..];

            if let Some(name_offset) = after_brace.find(enum_name) {
                let insert_pos =
                    enum_info.location.start_byte + closing_brace_pos + 1 + name_offset;

                return Fix::new(insert_pos, insert_pos, "G_GNUC_FLAG_ENUM ".to_string());
            }
        }

        Fix::new(
            enum_info.location.end_byte - 1,
            enum_info.location.end_byte - 1,
            " G_GNUC_FLAG_ENUM".to_string(),
        )
    }
}
