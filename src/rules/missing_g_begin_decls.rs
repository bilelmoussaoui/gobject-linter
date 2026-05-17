use gobject_ast::model::{Expression, PreprocessorDirective, TopLevelItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Category, Rule, Violation},
};

pub struct MissingGBeginDecls;

impl Rule for MissingGBeginDecls {
    fn name(&self) -> &'static str {
        "missing_g_begin_decls"
    }

    fn description(&self) -> &'static str {
        "Detect headers with missing or mismatched G_BEGIN_DECLS/G_END_DECLS"
    }

    fn category(&self) -> Category {
        Category::Correctness
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_header_files() {
            let decls_block = file.iter_all_items().find_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::GObjectDeclsBlock {
                    location,
                    ..
                }) => Some(location),
                _ => None,
            });

            if let Some(loc) = &decls_block {
                // GObjectDeclsBlock exists, verify G_END_DECLS is actually present
                let source = loc.source();
                let mut pos = loc.end_byte;
                while pos > 0 && source[pos - 1] != b'\n' {
                    pos -= 1;
                }
                let end_line = source[pos..loc.end_byte].trim_ascii_start();
                if !end_line.starts_with(b"G_END_DECLS") {
                    violations.push(self.violation_at(
                        path,
                        loc,
                        "G_BEGIN_DECLS without matching G_END_DECLS".to_string(),
                    ));
                }
                continue;
            }

            let orphan_begin = file.iter_all_items().find_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Call {
                    directive,
                    location,
                }) if directive == "G_BEGIN_DECLS" => Some(location),
                TopLevelItem::Expression(expr)
                    if matches!(expr.as_ref(), Expression::Identifier(id) if id.name == "G_BEGIN_DECLS") =>
                {
                    match expr.as_ref() {
                        Expression::Identifier(id) => Some(&id.location),
                        _ => None,
                    }
                }
                _ => None,
            });

            let orphan_end = file.iter_all_items().find_map(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Call {
                    directive,
                    location,
                }) if directive == "G_END_DECLS" => Some(location),
                TopLevelItem::Expression(expr)
                    if matches!(expr.as_ref(), Expression::Identifier(id) if id.name == "G_END_DECLS") =>
                {
                    match expr.as_ref() {
                        Expression::Identifier(id) => Some(&id.location),
                        _ => None,
                    }
                }
                _ => None,
            });

            if let Some(loc) = orphan_begin {
                violations.push(self.violation_at(
                    path,
                    loc,
                    "G_BEGIN_DECLS without matching G_END_DECLS".to_string(),
                ));
            }

            if let Some(loc) = orphan_end {
                violations.push(self.violation_at(
                    path,
                    loc,
                    "G_END_DECLS without matching G_BEGIN_DECLS".to_string(),
                ));
            }

            if orphan_begin.is_none()
                && orphan_end.is_none()
                && ast_context.is_public_header(path) == Some(true)
            {
                violations.push(self.violation(
                    path,
                    1,
                    1,
                    "Public header is missing G_BEGIN_DECLS/G_END_DECLS".to_string(),
                ));
            }
        }
    }
}
