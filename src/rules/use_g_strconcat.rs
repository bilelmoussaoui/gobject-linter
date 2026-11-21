use gobject_ast::model::{CallExpression, Expression, FileModel, FunctionDefItem};

use crate::{
    ast_context::AstContext,
    config::Config,
    rules::{Fix, Rule, Violation},
};

pub struct UseGStrconcat;

#[derive(Debug, PartialEq)]
enum FormatPiece {
    Literal(String),
    Placeholder,
}

/// Parse a printf format string (raw source text without quotes).
/// Returns `None` unless it contains at least one `%s` and every conversion
/// is a plain `%s` (no flags, width, precision or length modifiers).
fn parse_concat_format(raw: &str) -> Option<Vec<FormatPiece>> {
    let mut pieces = Vec::new();
    let mut literal = String::new();
    let mut has_placeholder = false;
    let mut chars = raw.chars();

    while let Some(c) = chars.next() {
        if c != '%' {
            literal.push(c);
            continue;
        }
        match chars.next() {
            Some('%') => literal.push('%'),
            Some('s') => {
                has_placeholder = true;
                if !literal.is_empty() {
                    pieces.push(FormatPiece::Literal(std::mem::take(&mut literal)));
                }
                pieces.push(FormatPiece::Placeholder);
            }
            _ => return None,
        }
    }

    if !has_placeholder {
        return None;
    }
    if !literal.is_empty() {
        pieces.push(FormatPiece::Literal(literal));
    }
    Some(pieces)
}

impl Rule for UseGStrconcat {
    fn name(&self) -> &'static str {
        "use_g_strconcat"
    }

    fn description(&self) -> &'static str {
        "Use g_strconcat() instead of g_strdup_printf() when the format string only contains %s placeholders"
    }

    fn long_description(&self) -> Option<&'static str> {
        Some(include_str!("../../docs/rules/use_g_strconcat.md"))
    }

    fn category(&self) -> crate::rules::Category {
        crate::rules::Category::Perf
    }

    fn fixable(&self) -> bool {
        true
    }

    fn check_func_impl(
        &self,
        _ast_context: &AstContext,
        config: &Config,
        func: &FunctionDefItem,
        file: &FileModel,
        violations: &mut Vec<Violation>,
    ) {
        for call in func.find_calls(&["g_strdup_printf"]) {
            self.check_call(file, call, config, violations);
        }
    }
}

impl UseGStrconcat {
    fn check_call(
        &self,
        file: &FileModel,
        call: &CallExpression,
        config: &Config,
        violations: &mut Vec<Violation>,
    ) {
        let Some(Expression::StringLiteral(lit)) = call.get_arg(0) else {
            return;
        };
        let Some(inner) = lit
            .value
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
        else {
            return;
        };
        // A remaining quote means concatenated string literals ("a" "b")
        if inner.contains('"') {
            return;
        }

        let Some(pieces) = parse_concat_format(inner) else {
            return;
        };

        // Require a matching argument for every %s
        let placeholders = pieces
            .iter()
            .filter(|p| matches!(p, FormatPiece::Placeholder))
            .count();
        if call.arguments.len() != placeholders + 1 {
            return;
        }

        let mut args: Vec<String> = Vec::with_capacity(pieces.len() + 1);
        let mut arg_index = 1;
        for piece in &pieces {
            match piece {
                FormatPiece::Literal(text) => args.push(format!("\"{text}\"")),
                FormatPiece::Placeholder => {
                    let Some(text) = call.get_arg_text(arg_index) else {
                        return;
                    };
                    args.push(text.to_string());
                    arg_index += 1;
                }
            }
        }
        args.push("NULL".to_string());

        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let replacement = config.style.format_call("g_strconcat", &arg_refs);
        let message =
            format!("Use {replacement} instead of g_strdup_printf() to concatenate strings");
        let fix = Fix::new(
            call.location.start_byte,
            call.location.end_byte,
            replacement,
        );

        violations.push(self.violation_with_fix_at(&file.path, &call.location, message, fix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal(text: &str) -> FormatPiece {
        FormatPiece::Literal(text.to_string())
    }

    #[test]
    fn parses_concatenation() {
        assert_eq!(
            parse_concat_format("%s%s"),
            Some(vec![FormatPiece::Placeholder, FormatPiece::Placeholder])
        );
        assert_eq!(
            parse_concat_format(r"%s: %s\n"),
            Some(vec![
                FormatPiece::Placeholder,
                literal(": "),
                FormatPiece::Placeholder,
                literal(r"\n"),
            ])
        );
    }

    #[test]
    fn unescapes_percent() {
        assert_eq!(
            parse_concat_format("100%% of %s"),
            Some(vec![literal("100% of "), FormatPiece::Placeholder])
        );
        // %%s is a literal '%' followed by 's', not a conversion
        assert_eq!(
            parse_concat_format("%%s %s"),
            Some(vec![literal("%s "), FormatPiece::Placeholder])
        );
    }

    #[test]
    fn rejects_other_conversions() {
        for fmt in ["%d", "%s %u", "%c%s", "%p"] {
            assert_eq!(parse_concat_format(fmt), None);
        }
    }

    #[test]
    fn rejects_modifiers() {
        for fmt in ["%10s", "%.3s", "%-*s", "%ls", "%1$s", "%s %"] {
            assert_eq!(parse_concat_format(fmt), None);
        }
    }

    #[test]
    fn rejects_no_placeholder() {
        for fmt in ["constant", "", "100%%"] {
            assert_eq!(parse_concat_format(fmt), None);
        }
    }
}
