use gobject_ast::model::{Expression, FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct UntranslatedString;

impl Rule for UntranslatedString {
    fn name(&self) -> &'static str {
        "untranslated_string"
    }

    fn description(&self) -> &'static str {
        "Detect user-visible strings that should be wrapped with gettext"
    }

    fn category(&self) -> Category {
        Category::Pedantic
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for call in func.find_calls_matching(|name| self.get_translatable_param(name).is_some()) {
            let Some(func_name) = call.function_name_str() else {
                continue;
            };
            let Some(arg_index) = self.get_translatable_param(func_name) else {
                continue;
            };
            let Some(arg) = call.arguments.get(arg_index) else {
                continue;
            };
            self.check_argument(arg, func_name, file, violations);
        }
    }
}

impl UntranslatedString {
    fn check_argument(
        &self,
        arg: &Expression,
        func_name: &str,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if let Expression::Call(call) = arg
            && let Some(name) = call.function_name_str()
            && matches!(name, "_" | "gettext" | "N_" | "g_dgettext" | "g_dpgettext2")
        {
            return;
        }

        // Check if it's a raw string literal
        if let Expression::StringLiteral(_) = arg {
            // Extract the string value without quotes
            if let Some(string_value) = arg.extract_string_value() {
                if string_value.trim().is_empty() {
                    return;
                }
                // Skip strings with no alphabetic characters (numbers,
                // punctuation, format specifiers, etc.)
                if !string_value.chars().any(char::is_alphabetic) {
                    return;
                }
            }

            let location = arg.location();
            violations.push(self.violation_at(
                &file.path,
                location,
                format!(
                    "User-visible string in {}() should be wrapped with _(\"...\")",
                    func_name
                ),
            ));
        }
    }

    /// Returns argument index for functions that take translatable strings
    fn get_translatable_param(&self, func_name: &str) -> Option<usize> {
        match func_name {
            "gtk_label_new" => Some(0),
            "gtk_label_set_text" | "gtk_label_set_markup" | "gtk_label_set_label" => Some(1),

            "gtk_button_new_with_label" | "gtk_button_new_with_mnemonic" => Some(0),
            "gtk_button_set_label" => Some(1),

            "gtk_window_set_title" => Some(1),

            "gtk_header_bar_set_title" | "gtk_header_bar_set_subtitle" => Some(1),

            "gtk_check_button_new_with_label" | "gtk_check_button_new_with_mnemonic" => Some(0),

            "gtk_radio_button_new_with_label" | "gtk_radio_button_new_with_mnemonic" => Some(1),

            "gtk_entry_set_placeholder_text" | "gtk_entry_set_text" => Some(1),

            "gtk_dialog_add_button" => Some(1),

            "gtk_message_dialog_new" => Some(4),
            "gtk_message_dialog_set_markup" => Some(1),

            "adw_message_dialog_new"
            | "adw_message_dialog_set_heading"
            | "adw_message_dialog_set_body"
            | "adw_message_dialog_set_body_use_markup" => Some(1),
            "adw_message_dialog_add_response" => Some(2),

            "adw_status_page_set_title" | "adw_status_page_set_description" => Some(1),

            "adw_toast_new" => Some(0),
            "adw_toast_set_title" | "adw_toast_set_button_label" => Some(1),

            "adw_preferences_group_set_title" | "adw_preferences_group_set_description" => Some(1),

            "adw_preferences_row_set_title" => Some(1),

            "adw_action_row_set_title" | "adw_action_row_set_subtitle" => Some(1),

            "adw_entry_row_set_title" => Some(1),

            "adw_combo_row_set_title" => Some(1),

            "adw_expander_row_set_title" | "adw_expander_row_set_subtitle" => Some(1),

            "adw_window_title_new" => Some(0),
            "adw_window_title_set_title" | "adw_window_title_set_subtitle" => Some(1),

            _ => None,
        }
    }
}
