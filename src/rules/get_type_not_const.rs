use gobject_ast::model::{ExportMacro, FileModel, FunctionDeclItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Fix, Rule, Violation},
};

const BAD_ATTRS: &[&str] = &["G_GNUC_CONST", "G_GNUC_PURE"];

pub struct GetTypeNotConst;

impl Rule for GetTypeNotConst {
    fn name(&self) -> &'static str {
        "get_type_not_const"
    }

    fn description(&self) -> &'static str {
        "Warn _get_type functions annotated with G_GNUC_CONST or G_GNUC_PURE"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/get_type_not_const.md"))
    }

    fn category(&self) -> Category {
        Category::Correctness
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_decl(
        &self,
        _ast_context: &AstContext,
        _config: &Config,
        func: &FunctionDeclItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        if !func.name.ends_with("_get_type") {
            return;
        }

        let Some(attr) = Self::find_bad_attr(func) else {
            return;
        };

        let message = format!(
            "'{}' is annotated with {} but _get_type functions have side effects on first call; remove the attribute",
            func.name, attr,
        );

        if let Some(fix) = Self::generate_fix(func, attr) {
            violations.push(self.violation_with_fix_at(&file.path, &func.location, message, fix));
        } else {
            violations.push(self.violation_at(&file.path, &func.location, message));
        }
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_all_files() {
            let ext = path.extension().and_then(|e| e.to_str());
            if ext == Some("c") || ext == Some("h") {
                for func in file.iter_function_declarations() {
                    self.check_func_decl(ast_context, config, func, file, violations);
                }
            }
        }
    }
}

impl GetTypeNotConst {
    fn find_bad_attr(func: &FunctionDeclItem) -> Option<&'static str> {
        for &attr in BAD_ATTRS {
            if func.macro_modifiers.iter().any(|m| m == attr) {
                return Some(attr);
            }
            if func
                .export_macros
                .iter()
                .any(|m| matches!(m, ExportMacro::Other(name) if name == attr))
            {
                return Some(attr);
            }
        }
        None
    }

    fn generate_fix(func: &FunctionDeclItem, attr: &str) -> Option<Fix> {
        let decl_text = func.location.as_str()?;
        let offset = decl_text.find(attr)?;
        let attr_start = func.location.start_byte + offset;
        let attr_end = attr_start + attr.len();

        // After params: delete leading whitespace + attr (`…(void) G_GNUC_CONST;`)
        if offset > 0 && decl_text.as_bytes()[offset - 1] == b' ' {
            return Some(Fix::delete(attr_start - 1, attr_end));
        }
        // Before return type: delete attr + trailing whitespace (`G_GNUC_CONST GType…`)
        if attr_end < func.location.end_byte
            && decl_text.as_bytes().get(offset + attr.len()) == Some(&b' ')
        {
            return Some(Fix::delete(attr_start, attr_end + 1));
        }
        Some(Fix::delete(attr_start, attr_end))
    }
}
