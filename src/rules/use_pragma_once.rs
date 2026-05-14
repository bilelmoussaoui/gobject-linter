use gobject_ast::model::{
    ConditionalKind, PragmaKind, PreprocessorDirective, SourceLocation, TopLevelItem,
};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UsePragmaOnce;

impl Rule for UsePragmaOnce {
    fn name(&self) -> &'static str {
        "use_pragma_once"
    }

    fn description(&self) -> &'static str {
        "Suggest #pragma once instead of traditional include guards"
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Style
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_all(
        &self,
        ast_context: &AstContext,
        _config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        for (path, file) in ast_context.iter_header_files() {
            // Check if the file already uses #pragma once
            let has_pragma_once = file.top_level_items.iter().any(|item| {
                matches!(
                    item,
                    TopLevelItem::Preprocessor(PreprocessorDirective::Pragma {
                        kind: PragmaKind::Once,
                        ..
                    })
                )
            });

            if has_pragma_once {
                continue; // Already using #pragma once
            }

            // Look for traditional include guard pattern
            if let Some((ifndef_loc, define_loc, endif_loc, guard_name)) =
                self.find_include_guard(&file.top_level_items)
            {
                // Build fixes:
                // 1. Replace #ifndef and #define lines with #pragma once
                // 2. Remove the #endif line (including any comment) at the end
                let mut fixes = Vec::new();

                // Fix 1: Delete #ifndef line (with following blank line if any)
                let ifndef_fix_loc =
                    ifndef_loc.with_byte_range(ifndef_loc.start_byte, ifndef_loc.start_byte);
                fixes.push(Fix::delete_line_and_trailing_blank(&ifndef_fix_loc));

                // Fix 2: Replace #define line with #pragma once
                let (define_start, define_end) = define_loc.find_line_bounds();
                fixes.push(Fix::new(define_start, define_end, "#pragma once\n"));

                // Fix 3: Remove the entire #endif line (with preceding blank line if any)
                fixes.push(Fix::delete_line_and_leading_blank(&endif_loc));

                violations.push(self.violation_with_fixes_at(
                    path,
                    &ifndef_loc,
                    format!("Use #pragma once instead of include guard '{}'", guard_name),
                    fixes,
                ));
            }
        }
    }
}

impl UsePragmaOnce {
    /// Find traditional include guard pattern
    /// Returns (ifndef_location, define_location, endif_location, guard_name)
    fn find_include_guard<'a>(
        &self,
        items: &'a [TopLevelItem],
    ) -> Option<(SourceLocation, SourceLocation, SourceLocation, &'a str)> {
        // The first non-comment item should be #ifndef (traditional include guard)
        items
            .iter()
            .find(|item| !matches!(item, TopLevelItem::Comment(_)))
            .and_then(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Conditional {
                    kind: ConditionalKind::Ifndef,
                    condition: Some(name),
                    body,
                    location,
                }) => {
                    // Found #ifndef - check it contains matching #define as first item
                    let define_loc = self.find_matching_define(body, name)?;

                    let endif_loc = location.with_byte_range(location.end_byte, location.end_byte);
                    let ifndef_loc = location.clone();

                    Some((ifndef_loc, define_loc, endif_loc, name.as_str()))
                }
                _ => None, // First item is not #ifndef
            })
    }

    /// Find matching #define inside the #ifndef body
    /// Returns the define location only if it's a guard (no value) and there's
    /// content after it
    fn find_matching_define(
        &self,
        body: &[TopLevelItem],
        guard_name: &str,
    ) -> Option<SourceLocation> {
        let non_comment_items: Vec<_> = body
            .iter()
            .filter(|item| !matches!(item, TopLevelItem::Comment(_)))
            .collect();

        if non_comment_items.len() < 2 {
            return None;
        }

        non_comment_items
            .first()
            .and_then(|item| match item {
                TopLevelItem::Preprocessor(PreprocessorDirective::Define {
                    name,
                    value,
                    location,
                }) => {
                    if name == guard_name && value.is_none() {
                        Some(location)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .cloned()
    }
}
