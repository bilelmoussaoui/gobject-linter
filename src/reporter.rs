use std::{collections::HashMap, env, io::IsTerminal, time::Duration};

use colored::*;

use crate::{
    config::{Config, RuleLevel},
    rules::Violation,
};

pub fn report_violations(violations: &[Violation], verbose: bool, config: &Config, duration: Duration) {
    // Check if we're outputting to a terminal
    let use_hyperlinks = std::io::stdout().is_terminal();

    if violations.is_empty() {
        if verbose {
            println!("{}", "No violations found!".green().bold());
        }
        return;
    }

    println!(
        "{}",
        format!("Found {} violation(s):", violations.len())
            .red()
            .bold()
    );
    println!();

    for violation in violations {
        // Create clickable link (or plain text if not a terminal)
        let file_link = create_clickable_link(
            &violation.file,
            violation.line,
            violation.column,
            &config.editor_url,
            use_hyperlinks,
        );

        println!("{}", file_link);

        // Show code snippet if available
        if let Some(ref snippet) = violation.snippet {
            // Add indentation to each line
            for line in snippet.lines() {
                println!("  {}", line.dimmed());
            }
        }

        let level_label = match violation.level {
            RuleLevel::Error => "error:".red().bold(),
            RuleLevel::Warn => "warning:".yellow().bold(),
            RuleLevel::Ignore => {
                unreachable!("Ignored violations should not be reported")
            }
        };
        println!("  {} {}", level_label, violation.message);
        println!("  {} {}", "rule:".blue(), violation.rule);
        println!();
    }

    println!(
        "{} violation(s) in {}",
        violations.len().to_string().yellow().bold(),
        format_duration(duration).dimmed(),
    );
}

/// Print a summary table of violation counts grouped by rule, sorted by count
/// descending. `fixable` maps rule name → whether the rule supports auto-fix.
pub fn report_summary(
    violations: &[Violation],
    fixable: &HashMap<&str, bool>,
    rule_timings: &[(&str, Duration)],
    duration: Duration,
) {
    if violations.is_empty() {
        println!("{}", "No violations found!".green().bold());
        return;
    }

    // Aggregate counts per rule and capture level (all violations of same rule
    // should have same level)
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut levels: HashMap<&str, RuleLevel> = HashMap::new();
    for v in violations {
        *counts.entry(v.rule).or_insert(0) += 1;
        levels.entry(v.rule).or_insert(v.level);
    }

    // Build timing lookup
    let timings: HashMap<&str, Duration> = rule_timings.iter().copied().collect();

    // Build sorted rows: (rule, count, level, fixable, time), descending by count.
    let mut rows: Vec<(&str, usize, RuleLevel, bool, String)> = counts
        .iter()
        .map(|(&rule, &count)| {
            let time_str = timings
                .get(rule)
                .map(|d| format_duration(*d))
                .unwrap_or_default();
            (
                rule,
                count,
                *levels.get(rule).unwrap(),
                *fixable.get(rule).unwrap_or(&false),
                time_str,
            )
        })
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    // Column widths — at least wide enough for the header labels.
    let count_w = rows
        .iter()
        .map(|(_, c, ..)| c.to_string().len())
        .max()
        .unwrap_or(0)
        .max("Count".len());
    let level_w = "Level".len();
    let rule_w = rows
        .iter()
        .map(|(r, ..)| r.len())
        .max()
        .unwrap_or(0)
        .max("Rule".len());
    let fix_w = "Autofix".len();
    let time_w = rows
        .iter()
        .map(|(.., t)| t.len())
        .max()
        .unwrap_or(0)
        .max("Time".len());

    let top = format!(
        "┌{:─<cw$}┬{:─<rw$}┬{:─<lw$}┬{:─<fw$}┬{:─<tw$}┐",
        "", "", "", "", "",
        cw = count_w + 2, rw = rule_w + 2, lw = level_w + 2, fw = fix_w + 2, tw = time_w + 2,
    );
    let sep = format!(
        "├{:─<cw$}┼{:─<rw$}┼{:─<lw$}┼{:─<fw$}┼{:─<tw$}┤",
        "", "", "", "", "",
        cw = count_w + 2, rw = rule_w + 2, lw = level_w + 2, fw = fix_w + 2, tw = time_w + 2,
    );
    let bot = format!(
        "└{:─<cw$}┴{:─<rw$}┴{:─<lw$}┴{:─<fw$}┴{:─<tw$}┘",
        "", "", "", "", "",
        cw = count_w + 2, rw = rule_w + 2, lw = level_w + 2, fw = fix_w + 2, tw = time_w + 2,
    );

    println!("{}", top);
    println!(
        "│ {:<cw$} │ {:<rw$} │ {:<lw$} │ {:<fw$} │ {:<tw$} │",
        "Count".bold(),
        "Rule".bold(),
        "Level".bold(),
        "Autofix".bold(),
        "Time".bold(),
        cw = count_w, rw = rule_w, lw = level_w, fw = fix_w, tw = time_w,
    );
    println!("{}", sep);

    for (rule, count, level, is_fixable, time_str) in &rows {
        let count_str = count.to_string().yellow().to_string();
        let (level_str, level_len) = match level {
            RuleLevel::Error => ("error".red().to_string(), 5),
            RuleLevel::Warn => ("warn".yellow().to_string(), 4),
            RuleLevel::Ignore => {
                unreachable!("Ignored violations should not be in summary")
            }
        };
        let rule_str = rule.cyan().to_string();
        let fix_str = if *is_fixable {
            "Yes".green().to_string()
        } else {
            "No".dimmed().to_string()
        };
        let time_colored = time_str.dimmed().to_string();

        let count_pad = count_w - count.to_string().len();
        let level_pad = level_w - level_len;
        let rule_pad = rule_w - rule.len();
        let fix_pad = fix_w - if *is_fixable { 3 } else { 2 };
        let time_pad = time_w - time_str.len();

        println!(
            "│ {}{} │ {}{} │ {}{} │ {}{} │ {}{} │",
            count_str, " ".repeat(count_pad),
            rule_str, " ".repeat(rule_pad),
            level_str, " ".repeat(level_pad),
            fix_str, " ".repeat(fix_pad),
            time_colored, " ".repeat(time_pad),
        );
    }

    println!("{}", bot);

    // Calculate total from the counts table to ensure accuracy
    let total_count: usize = counts.values().sum();

    println!(
        "  {} violation(s) across {} rule(s) in {}",
        total_count.to_string().yellow().bold(),
        rows.len().to_string().yellow().bold(),
        format_duration(duration).dimmed(),
    );
}
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else {
        format!("{:.2}s", secs)
    }
}

fn create_clickable_link(
    file_path: &std::path::Path,
    line: usize,
    column: usize,
    editor_url_template: &Option<String>,
    use_hyperlinks: bool,
) -> String {
    // Convert to absolute path if relative
    let abs_path = if file_path.is_absolute() {
        file_path
    } else {
        match env::current_dir() {
            Ok(cwd) => &cwd.join(file_path),
            Err(_) => file_path,
        }
    };

    // Format: file:line:column
    let location = format!("{}:{}:{}", abs_path.display(), line, column);

    if !use_hyperlinks {
        // Plain text output for pipes, redirects, etc. - no colors, no hyperlinks
        return location;
    }

    // Use configured editor URL or default
    let file_url = if let Some(template) = editor_url_template {
        template
            .replace("{path}", &abs_path.display().to_string())
            .replace("{line}", &line.to_string())
            .replace("{column}", &column.to_string())
    } else {
        // Default: just use file:// protocol
        format!("file://{}", abs_path.display())
    };

    // OSC 8 hyperlink escape sequence with colored location
    let hyperlink = format!(
        "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
        file_url,
        location.cyan()
    );

    hyperlink
}
