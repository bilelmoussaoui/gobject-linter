use std::{collections::HashMap, path::Path};

use gobject_ast::FileModel;

/// Parse inline ignore directives from comments
/// Returns a map of (line_number -> set of ignored rules)
pub fn parse_ignore_directives(file_model: &FileModel) -> HashMap<usize, Vec<String>> {
    let mut ignores: HashMap<usize, Vec<String>> = HashMap::new();

    // We need to parse the source to find comments
    // For now, we'll scan the raw source text for comment patterns
    // This is a simple implementation that looks for:
    // /* gobject-linter-ignore: rule_name */
    // /* gobject-linter-ignore-next-line: rule_name */
    // // gobject-linter-ignore: rule_name
    // // gobject-linter-ignore-next-line: rule_name

    if let Ok(source_str) = std::str::from_utf8(&file_model.source) {
        let lines: Vec<&str> = source_str.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1; // 1-indexed

            // Check for ignore directives
            if let Some(rules) = parse_ignore_comment(line, false) {
                // Ignore on current line
                ignores.entry(line_num).or_default().extend(rules);
            }

            if let Some(rules) = parse_ignore_comment(line, true) {
                // Ignore on next line
                let next_line = line_num + 1;
                ignores.entry(next_line).or_default().extend(rules);
            }
        }
    }

    ignores
}

/// Parse a single line for ignore directive
/// Returns Some(rules) if found, None otherwise
fn parse_ignore_comment(line: &str, next_line: bool) -> Option<Vec<String>> {
    // Accept both the current name and the legacy "goblint" prefix
    let directives: &[&str] = if next_line {
        &[
            "gobject-linter-ignore-next-line:",
            "goblint-ignore-next-line:",
        ]
    } else {
        &["gobject-linter-ignore:", "goblint-ignore:"]
    };

    let (directive, idx) = directives
        .iter()
        .find_map(|d| line.find(d).map(|i| (*d, i)))?;

    let rest = line[idx + directive.len()..]
        .trim()
        .trim_end_matches("*/")
        .trim();

    let rules: Vec<String> = rest
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if !rules.is_empty() { Some(rules) } else { None }
}

/// Check if a violation should be ignored based on inline directives
pub fn should_ignore_violation(
    file_path: &Path,
    line: usize,
    rule: &str,
    ignore_map: &HashMap<&Path, HashMap<usize, Vec<String>>>,
) -> bool {
    if let Some(file_ignores) = ignore_map.get(file_path)
        && let Some(ignored_rules) = file_ignores.get(&line)
    {
        // Check if this specific rule is ignored
        if ignored_rules.contains(&rule.to_string()) {
            return true;
        }
        // Check for wildcard ignore (goblint-ignore: all)
        if ignored_rules.iter().any(|r| r == "all" || r == "*") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ignore_comment() {
        // Same line ignore
        assert_eq!(
            parse_ignore_comment("  /* gobject-linter-ignore: rule_name */", false),
            Some(vec!["rule_name".to_string()])
        );

        // Multiple rules
        assert_eq!(
            parse_ignore_comment("  /* gobject-linter-ignore: rule1, rule2 */", false),
            Some(vec!["rule1".to_string(), "rule2".to_string()])
        );

        // Next line ignore
        assert_eq!(
            parse_ignore_comment("  /* gobject-linter-ignore-next-line: rule_name */", true),
            Some(vec!["rule_name".to_string()])
        );

        // C++ style comment
        assert_eq!(
            parse_ignore_comment("  // gobject-linter-ignore: rule_name", false),
            Some(vec!["rule_name".to_string()])
        );

        // No directive
        assert_eq!(parse_ignore_comment("  /* regular comment */", false), None);
    }
}
