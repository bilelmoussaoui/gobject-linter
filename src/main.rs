use std::{
    collections::{HashMap, HashSet},
    io::{IsTerminal, Read},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use gobject_linter::{
    ast_context, config, config::OutputFormat, fixer, output, reporter, rules::Category, scanner,
};
use indicatif::{ProgressBar, ProgressStyle};
use unidiff::PatchSet;

#[derive(Parser, Debug)]
#[command(name = "gobject-linter")]
#[command(about = "A fast tree-sitter-based linter for GObject/C code", long_about = None)]
struct Args {
    /// Directory to scan for C files
    #[arg(value_name = "DIRECTORY", default_value = ".")]
    directory: PathBuf,

    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", default_value = concat!(env!("CARGO_PKG_NAME"), ".toml"))]
    config: PathBuf,

    /// Ignore files matching glob patterns (e.g., "tests/**", "vendor/**")
    #[arg(short, long, value_name = "PATTERN")]
    ignore: Vec<String>,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,

    /// List all available lint rules
    #[arg(long)]
    list_rules: bool,

    /// Enable only specific rules (can be repeated, overrides config)
    #[arg(long, value_name = "RULE")]
    only: Vec<String>,

    /// Disable specific rules (can be repeated, overrides config)
    #[arg(long, value_name = "RULE")]
    exclude: Vec<String>,

    /// Enable only rules from this category (e.g., correctness, style, perf)
    #[arg(long, value_name = "CATEGORY")]
    category: Option<Category>,

    /// Output format
    #[arg(long, value_enum)]
    format: Option<OutputFormat>,

    /// Automatically apply fixes for violations
    #[arg(long)]
    fix: bool,

    /// Print a summary table of violation counts grouped by rule
    #[arg(long)]
    summary: bool,

    /// Set minimum GLib version (e.g., "2.76") - disables rules requiring newer
    /// versions
    #[arg(long, value_name = "VERSION", value_parser = parse_glib_version_arg)]
    min_glib_version: Option<(u32, u32)>,

    /// Target MSVC-compatible code (disables g_auto* rules, enables
    /// no_g_auto_macros)
    #[arg(long)]
    msvc_compatible: bool,

    /// Only report violations on lines changed in this unified diff (use `-`
    /// to read from stdin). Useful for CI to report only on PR changes.
    #[arg(long, value_name = "FILE")]
    diff: Option<PathBuf>,
}

/// Parse GLib version string for clap
fn parse_glib_version_arg(s: &str) -> Result<(u32, u32), String> {
    config::parse_glib_version(s).ok_or_else(|| {
        format!(
            "Invalid GLib version format: '{}'. Expected format: 'major.minor' (e.g., '2.76')",
            s
        )
    })
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_line_number(true)
        .init();

    // Load configuration
    let config_path =
        if !args.config.exists() && args.config == std::path::Path::new("gobject-linter.toml") {
            let legacy = std::path::Path::new("goblint.toml");
            if legacy.exists() {
                legacy.to_path_buf()
            } else {
                args.config.clone()
            }
        } else {
            args.config.clone()
        };
    let mut config = config::Config::load(&config_path)?;

    let format = args.format.or(config.format).unwrap_or_default();

    // Auto-disable colors for machine-readable formats or when not a terminal
    if matches!(
        format,
        OutputFormat::Json | OutputFormat::Sarif | OutputFormat::Gcc
    ) {
        // Machine-readable formats never use colors
        colored::control::set_override(false);
    } else {
        if !std::io::stdout().is_terminal() {
            colored::control::set_override(false);
        }
    }

    // Merge CLI ignore patterns with config
    config.ignore.extend(args.ignore.clone());

    // Apply --min-glib-version if specified (overrides config)
    if let Some(version) = args.min_glib_version {
        config.min_glib_version = Some(version);
    }

    // Apply --msvc-compatible if specified (overrides config)
    if args.msvc_compatible {
        config.msvc_compatible = true;
    }

    // Apply --only filter if specified
    if !args.only.is_empty() {
        config.enable_only_rules(&args.only)?;
    }

    // Apply --exclude filter if specified
    if !args.exclude.is_empty() {
        config.disable_rules(&args.exclude)?;
    }

    // Apply --category filter if specified
    if let Some(category) = args.category {
        config.filter_by_category(category)?;
    }

    // Handle --list-rules
    if args.list_rules {
        match format {
            OutputFormat::Json => {
                println!("{}", scanner::list_all_rules_json(&config));
            }
            _ => {
                scanner::list_all_rules(&config);
            }
        }
        return Ok(());
    }

    // Canonicalize directory path for consistent path handling
    let project_root = args
        .directory
        .canonicalize()
        .unwrap_or(args.directory.clone());

    // Build ignore matcher
    let ignore_matcher = config.build_ignore_matcher()?;

    // Create spinner for progress
    let spinner = if args.verbose {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        sp.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(sp)
    } else {
        None
    };

    // Get header visibility from meson introspection (for dead code analysis)
    if let Some(ref sp) = spinner {
        sp.set_message("Running meson introspection...");
    }
    let meson_headers =
        gobject_linter::meson::get_header_sets(&project_root, config.build_dir.as_deref())
            .ok()
            .flatten();

    if args.verbose {
        if let Some(ref h) = meson_headers {
            println!(
                "Meson introspection: {} GIR headers, {} installed headers",
                h.gir.len(),
                h.installed.len()
            );
        } else {
            println!(
                "Meson introspection not available - proceeding without public/private distinction"
            );
        }
    }

    let analysis_start = std::time::Instant::now();

    // Build AST-based context
    if let Some(ref sp) = spinner {
        sp.set_message("Parsing files...");
    }
    let ast_context = ast_context::AstContext::build_with_ignore(
        &project_root,
        &ignore_matcher,
        spinner.as_ref(),
        meson_headers,
    )?;
    let parse_duration = analysis_start.elapsed();

    // Run AST-based rules
    let scan_start = std::time::Instant::now();
    let (mut violations, rule_timings) = scanner::scan_with_ast(
        &ast_context,
        &config,
        &project_root,
        spinner.as_ref(),
        !args.summary && !args.fix,
    )?;
    let scan_duration = scan_start.elapsed();
    let analysis_duration = parse_duration + scan_duration;

    if let Some(sp) = spinner {
        sp.finish_and_clear();
    }

    // Filter violations to changed lines when a diff is provided
    if let Some(diff_path) = &args.diff {
        let diff_content = if diff_path == std::path::Path::new("-") {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        } else {
            std::fs::read_to_string(diff_path)?
        };

        let mut patch = PatchSet::new();
        let _ = patch.parse(&diff_content);

        // Diff paths are relative to the git root, which may differ from project_root
        let git_root = {
            let mut dir = project_root.as_path();
            loop {
                if dir.join(".git").exists() {
                    break dir.to_path_buf();
                }
                match dir.parent() {
                    Some(p) => dir = p,
                    None => break project_root.clone(),
                }
            }
        };

        let mut changed_lines: HashMap<std::path::PathBuf, HashSet<usize>> = HashMap::new();
        for file in patch {
            let path = git_root.join(file.path().trim_start_matches("b/"));
            let lines = changed_lines.entry(path).or_default();
            for hunk in file {
                for line in hunk {
                    if line.is_added()
                        && let Some(line_no) = line.target_line_no
                    {
                        lines.insert(line_no);
                    }
                }
            }
        }

        violations.retain(|v| {
            changed_lines
                .get(&v.file)
                .is_some_and(|lines| lines.contains(&v.line))
        });
    }

    if args.verbose {
        let total_functions: usize = ast_context
            .project
            .files
            .values()
            .map(|f| f.iter_function_declarations().count() + f.iter_function_definitions().count())
            .sum();
        let total_gobject_types: usize = ast_context
            .project
            .files
            .values()
            .map(|f| f.iter_all_gobject_types().count())
            .sum();
        println!(
            "Parsed {} files, {} functions, {} GObject types in {}",
            ast_context.project.files.len(),
            total_functions,
            total_gobject_types,
            reporter::format_duration(parse_duration),
        );
    }

    // Apply fixes if --fix was passed
    if args.fix {
        // Check if any enabled rules are fixable
        let rules = scanner::create_all_rules(&config);
        let has_fixable_rules = rules
            .iter()
            .any(|entry| entry.level.is_enabled() && entry.rule.fixable());

        if !has_fixable_rules {
            eprintln!(
                "Warning: --fix was specified but no enabled rules are auto-fixable.\n\
                 Run `goblin --list-rules` to see which rules support auto-fix."
            );
        } else {
            let fixed_count = fixer::apply_fixes(&violations)?;
            println!("Fixed {} violation(s)", fixed_count);
        }

        // Don't exit with error code when we fixed things
        return Ok(());
    }

    // Summary table mode
    if args.summary {
        let rules = scanner::create_all_rules(&config);
        let fixable: std::collections::HashMap<&str, bool> = rules
            .iter()
            .map(|e| (e.rule.name(), e.rule.fixable()))
            .collect();
        reporter::report_summary(&violations, &fixable, &rule_timings, analysis_duration);
        let has_errors = violations.iter().any(|v| v.level.is_error());
        if has_errors {
            std::process::exit(1);
        }
        return Ok(());
    }

    // Output violations in the requested format
    match format {
        OutputFormat::Text => {
            reporter::report_violations(&violations, args.verbose, &config, analysis_duration);
        }
        OutputFormat::Json => {
            let json_output = serde_json::to_string_pretty(&violations)
                .expect("Failed to serialize violations to JSON");
            println!("{}", json_output);
        }
        OutputFormat::Sarif => {
            let sarif_output = output::sarif::generate_sarif(&violations, &config, &project_root);
            println!("{}", sarif_output);
        }
        OutputFormat::Gcc => {
            output::gcc::generate_gcc(&violations);
        }
        OutputFormat::GitlabCodequality => {
            let json =
                output::gitlab_codequality::generate_gitlab_codequality(&violations, &project_root);
            println!("{}", json);
        }
    }

    // Exit with error code only if there are error-level violations (not warnings)
    let has_errors = violations.iter().any(|v| v.level.is_error());
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
