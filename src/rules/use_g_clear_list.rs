use gobject_ast::Statement;

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGClearList;

impl Rule for UseGClearList {
    fn name(&self) -> &'static str {
        "use_g_clear_list"
    }

    fn description(&self) -> &'static str {
        "Suggest g_clear_list/g_clear_slist instead of manual g_list_free/g_slist_free and NULL assignment"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Complexity
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &gobject_ast::top_level::FunctionDefItem,
        file: &gobject_ast::FileModel,
        violations: &mut Vec<Violation>,
    ) {
        Statement::walk_pairs(&func.body_statements, &mut |first, second| {
            let Some((var_name, list_type)) = self.extract_list_free(first, &file.source) else {
                return;
            };
            if !second.is_null_assignment_to(var_name, &file.source) {
                return;
            }
            let clear_fn = if list_type == "GList" {
                "g_clear_list"
            } else {
                "g_clear_slist"
            };
            let replacement = format!("{} (&{}, NULL);", clear_fn, var_name);
            let base_type = if list_type == "GList" {
                "g_list"
            } else {
                "g_slist"
            };
            let message =
                format!("Use {replacement} instead of {base_type}_free and NULL assignment");
            let second_end = second.location().find_semicolon_end(&file.source);
            let fixes = vec![
                Fix::delete_line(first.location(), &file.source),
                Fix::new(second.location().start_byte, second_end, replacement),
            ];
            violations.push(self.violation_with_fixes(
                &file.path,
                first.location().line,
                first.location().column,
                message,
                fixes,
            ));
        });
    }
}

impl UseGClearList {
    fn extract_list_free<'a>(
        &self,
        stmt: &Statement,
        source: &'a [u8],
    ) -> Option<(&'a str, &'static str)> {
        let call = stmt.extract_call()?;

        let func_name = call.function_name_str()?;
        let list_type = match func_name {
            "g_list_free" => "GList",
            "g_slist_free" => "GSList",
            _ => return None,
        };

        if call.arguments.is_empty() {
            return None;
        }

        let var_name = call.get_arg_text(0, source)?;
        Some((var_name, list_type))
    }
}
