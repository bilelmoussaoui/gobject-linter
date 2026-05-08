use std::{collections::HashSet, path::Path};

use anyhow::Result;
use colored::Colorize;
use indicatif::ProgressBar;
use rayon::prelude::*;
use serde::Serialize;

use crate::{
    ast_context::AstContext,
    config::{Config, RuleConfig, RuleLevel},
    inline_ignore,
    rules::{ConfigOption, *},
};

pub type ScanResult = Result<(Vec<Violation>, Vec<(&'static str, std::time::Duration)>)>;

/// Extract a source snippet from in-memory source bytes at the given line with
/// context
fn get_source_snippet(source: &[u8], line: usize) -> Option<String> {
    let content = std::str::from_utf8(source).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    if line == 0 || line > lines.len() {
        return None;
    }

    // Get 7 lines before and 3 lines after for context (11 lines total)
    let start_line = line.saturating_sub(8); // -1 for 0-indexing, -7 for context
    let end_line = (line + 3).min(lines.len());

    let mut snippet_lines = Vec::new();
    let mut last_was_collapsed = false;

    for (i, line_text) in lines.iter().enumerate().take(end_line).skip(start_line) {
        let trimmed = line_text.trim();
        let is_target_line = i + 1 == line;

        // Check if line is just braces/whitespace (but always show target line)
        let is_noise = !is_target_line && matches!(trimmed, "" | "{" | "}" | "{}" | "};");

        if is_noise {
            // Collapse consecutive noise lines into ...
            if !last_was_collapsed {
                snippet_lines.push("...".to_string());
                last_was_collapsed = true;
            }
        } else {
            let prefix = if is_target_line { ">" } else { "" };
            snippet_lines.push(format!("{}{}", prefix, line_text));
            last_was_collapsed = false;
        }
    }

    Some(snippet_lines.join("\n"))
}

/// Populate snippets for violations that don't have them
fn populate_snippets(violations: &mut [Violation], ast_context: &AstContext) {
    for violation in violations.iter_mut() {
        if violation.snippet.is_none()
            && let Some(file) = ast_context.project.files.get(&violation.file)
        {
            violation.snippet = get_source_snippet(&file.source, violation.line);
        }
    }
}

/// Filter violations in-place based on per-rule ignore patterns
fn filter_violations_in_place(
    violations: &mut Vec<Violation>,
    project_root: &Path,
    config: &Config,
    rule_config: &RuleConfig,
) -> Result<()> {
    let ignore_matcher = config.build_rule_ignore_matcher(rule_config)?;

    violations.retain(|v| {
        let relative_path = v.file.strip_prefix(project_root).unwrap_or(&v.file);
        !ignore_matcher.is_match(relative_path)
    });

    Ok(())
}

pub struct RuleEntry<'a> {
    pub rule: Box<dyn Rule>,
    pub level: RuleLevel,
    pub rule_config: &'a RuleConfig,
    pub min_glib_version: (u32, u32),
    pub requires_auto_cleanup: bool,
    /// Rule is disabled by default; user must explicitly enable it in config or
    /// via --only
    pub opt_in: bool,
}

/// Macro to define all rules in execution order with their minimum GLib version
/// requirements and MSVC compatibility
/// Format: (config_field, RuleType, min_major, min_minor,
/// requires_auto_cleanup)
/// - requires_auto_cleanup: true if rule suggests g_auto* macros (disabled when
///   msvc_compatible=true)
#[macro_export]
macro_rules! for_each_rule {
    ($callback:ident) => {
        $callback! {
            // (config_field, RuleType, min_major, min_minor, requires_auto_cleanup, opt_in)
            // opt_in = true: rule defaults to ignore, user must explicitly enable it
            (dead_code, DeadCode, 2, 0, false, true),
            (include_order, IncludeOrder, 2, 0, false, false),
            (inconsistent_function_signature, InconsistentFunctionSignature, 2, 0, false, false),
            (use_pragma_once, UsePragmaOnce, 2, 0, false, false),
            (missing_implementation, MissingImplementation, 2, 0, false, false),
            (missing_autoptr_cleanup, MissingAutoptrCleanup, 2, 0, false, false),
            (missing_export_macro, MissingExportMacro, 2, 0, false, true),
            (no_g_auto_macros, NoGAutoMacros, 2, 0, false, false),
            (deprecated_add_private, DeprecatedAddPrivate, 2, 0, false, false),
            (matching_declare_define, MatchingDeclareDefine, 2, 70, false, false),
            (use_g_new, UseGNew, 2, 0, false, false),
            (use_g_object_class_install_properties, UseGObjectClassInstallProperties, 2, 26, false, false),
            (use_g_settings_typed, UseGSettingsTyped, 2, 26, false, false),
            (use_g_value_set_static_string, UseGValueSetStaticString, 2, 0, false, false),
            (use_g_variant_new_typed, UseGVariantNewTyped, 2, 24, false, false),
            (strcmp_explicit_comparison, StrcmpExplicitComparison, 2, 0, false, false),
            (type_style, TypeStyle, 2, 0, false, false),
            (use_g_strcmp0, UseGStrcmp0, 2, 16, false, false),
            (use_clear_functions, UseClearFunctions, 2, 0, false, false),
            (use_explicit_default_flags, UseExplicitDefaultFlags, 2, 0, false, false),
            (g_param_spec_null_nick_blurb, GParamSpecNullNickBlurb, 2, 0, false, false),
            (g_param_spec_static_strings, GParamSpecStaticStrings, 2, 0, false, false),
            (property_canonical_name, PropertyCanonicalName, 2, 0, false, false),
            (g_error_init, GErrorInit, 2, 0, false, false),
            (g_error_leak, GErrorLeak, 2, 0, false, false),
            (g_source_id_not_stored, GSourceIdNotStored, 2, 0, false, false),
            (property_enum_convention, PropertyEnumConvention, 2, 0, false, false),
            (property_enum_coverage, PropertyEnumCoverage, 2, 0, false, false),
            (property_switch_exhaustiveness, PropertySwitchExhaustiveness, 2, 0, false, false),
            (signal_canonical_name, SignalCanonicalName, 2, 0, false, false),
            (signal_enum_coverage, SignalEnumCoverage, 2, 0, false, false),
            (g_object_virtual_methods_chain_up, GObjectVirtualMethodsChainUp, 2, 0, false, false),
            (g_task_source_tag, GTaskSourceTag, 2, 36, false, false),
            (unnecessary_null_check, UnnecessaryNullCheck, 2, 0, false, false),
            (use_g_set_object, UseGSetObject, 2, 44, false, false),
            (use_g_set_str, UseGSetStr, 2, 76, false, false),
            (use_g_autoptr_error, UseGAutoptrError, 2, 44, true, false),
            (use_g_autoptr_goto_cleanup, UseGAutoptrGotoCleanup, 2, 44, true, false),
            (use_g_autoptr_inline_cleanup, UseGAutoptrInlineCleanup, 2, 44, true, false),
            (use_g_autofree, UseGAutofree, 2, 44, true, false),
            (use_g_autolist, UseGAutolist, 2, 44, true, false),
            (use_g_bytes_unref_to_data, UseGBytesUnrefToData, 2, 32, false, false),
            (use_g_file_load_bytes, UseGFileLoadBytes, 2, 56, false, false),
            (use_g_gnuc_flag_enum, UseGGnucFlagEnum, 2, 87, false, false),
            (use_g_object_new_with_properties, UseGObjectNewWithProperties, 2, 0, false, false),
            (use_g_object_notify_by_pspec, UseGObjectNotifyByPspec, 2, 26, false, false),
            (use_g_string_free_and_steal, UseGStringFreeAndSteal, 2, 76, false, false),
            (use_g_source_once, UseGSourceOnce, 2, 74, false, false),
            (use_g_source_constants, UseGSourceConstants, 2, 0, false, false),
            (use_g_steal_pointer, UseGStealPointer, 2, 0, false, false),
            (use_g_str_has_prefix_suffix, UseGStrHasPrefixSuffix, 2, 0, false, false),
            (use_g_ascii_functions, UseGAsciiFunctions, 2, 0, false, false),
            (use_g_strlcpy, UseGStrlcpy, 2, 0, false, false),
            (untranslated_string, UntranslatedString, 2, 0, false, false),
            (gi_missing_since, GiMissingSince, 2, 0, false, true),
            (gi_not_bindings_friendly, GiNotBindingsFriendly, 2, 0, false, true),
        }
    };
}

macro_rules! impl_create_all_rules {
    ($(($config_field:ident, $rule_type:ident, $major:literal, $minor:literal, $requires_auto_cleanup:literal, $opt_in:literal)),* $(,)?) => {
        /// Create all rule instances in execution order
        pub fn create_all_rules<'a>(config: &'a Config) -> Vec<RuleEntry<'a>> {
            vec![
                $(
                    RuleEntry {
                        rule: Box::new($rule_type),
                        level: if is_rule_compatible(config, $major, $minor) {
                            let default_level = if $opt_in {
                                RuleLevel::Ignore
                            } else {
                                config.default_level.unwrap_or(RuleLevel::Warn)
                            };
                            let configured = config.rules.$config_field.level.unwrap_or(default_level);
                            apply_msvc_compatibility(config, stringify!($config_field), $requires_auto_cleanup, configured)
                        } else {
                            RuleLevel::Ignore
                        },
                        rule_config: &config.rules.$config_field,
                        min_glib_version: ($major, $minor),
                        requires_auto_cleanup: $requires_auto_cleanup,
                        opt_in: $opt_in,
                    },
                )*
            ]
        }
    };
}

/// Check if a rule is compatible with the configured minimum GLib version
fn is_rule_compatible(config: &Config, required_major: u32, required_minor: u32) -> bool {
    if let Some((major, minor)) = config.min_glib_version {
        // Compare versions: config version must be >= required version
        (major > required_major) || (major == required_major && minor >= required_minor)
    } else {
        // No minimum version set, all rules are compatible
        true
    }
}

/// Apply MSVC compatibility overrides to rule level
fn apply_msvc_compatibility(
    config: &Config,
    rule_name: &str,
    requires_auto_cleanup: bool,
    configured_level: RuleLevel,
) -> RuleLevel {
    match (rule_name, config.msvc_compatible) {
        ("no_g_auto_macros", false) => return RuleLevel::Ignore,
        ("no_g_auto_macros", true) => return RuleLevel::Error,
        (_, false) => return configured_level,
        // Continue
        (_, true) => (),
    }

    // Disable all rules that require auto cleanup attributes
    if requires_auto_cleanup {
        return RuleLevel::Ignore;
    }

    configured_level
}

for_each_rule!(impl_create_all_rules);

/// Validate that all rule names in inline ignore directives are valid
/// Returns a list of warnings about unknown rules
fn validate_inline_ignores(
    inline_ignores: &std::collections::HashMap<
        &Path,
        std::collections::HashMap<usize, Vec<String>>,
    >,
    rules: &[RuleEntry<'_>],
    project_root: &Path,
) -> Vec<String> {
    let mut warnings = Vec::new();

    // Collect all valid rule names
    let valid_rules: HashSet<String> = rules
        .iter()
        .map(|entry| entry.rule.name().to_string())
        .collect();

    // Check each file's ignore directives
    for (file_path, file_ignores) in inline_ignores {
        for (line_num, ignored_rules) in file_ignores {
            for rule_name in ignored_rules {
                // Skip wildcards
                if rule_name == "all" || rule_name == "*" {
                    continue;
                }

                // Check if rule exists
                if !valid_rules.contains(rule_name) {
                    let relative_path = file_path.strip_prefix(project_root).unwrap_or(file_path);
                    let warning = format!(
                        "{}:{}:1: {} Unknown rule '{}' in ignore directive",
                        relative_path.display(),
                        line_num,
                        "warning:".yellow(),
                        rule_name
                    );
                    warnings.push(warning);
                }
            }
        }
    }

    warnings
}

/// New AST-based scanner - much simpler than the old one!
pub fn scan_with_ast(
    ast_context: &AstContext,
    config: &Config,
    project_root: &Path,
    spinner: Option<&ProgressBar>,
    generate_snippets: bool,
) -> ScanResult {
    let mut violations = Vec::new();

    // Parse inline ignore directives from all files
    let inline_ignores: std::collections::HashMap<
        &Path,
        std::collections::HashMap<usize, Vec<String>>,
    > = ast_context
        .project
        .files
        .iter()
        .map(|(path, file)| {
            let ignores = inline_ignore::parse_ignore_directives(file);
            (path.as_path(), ignores)
        })
        .collect();

    // Register all rules in execution order
    let rules = create_all_rules(config);

    // Validate that all rule names in ignore directives are valid
    let warnings = validate_inline_ignores(&inline_ignores, &rules, project_root);
    for warning in warnings {
        eprintln!("{}", warning);
    }

    // Warn about unrecognized rule config options
    for entry in &rules {
        let known: HashSet<&str> = entry.rule.config_options().iter().map(|o| o.name).collect();
        for key in entry.rule_config.options.keys() {
            if !known.contains(key.as_str()) {
                eprintln!(
                    "{}: unknown option '{}' for rule '{}'",
                    "warning".yellow(),
                    key,
                    entry.rule.name()
                );
            }
        }
    }

    if let Some(sp) = spinner {
        sp.set_message("Running linter rules...");
    }

    // Run all rules in parallel — each gets its own violations vec
    let per_rule: Vec<(Result<Vec<Violation>>, &str, std::time::Duration)> = rules
        .par_iter()
        .enumerate()
        .map(|(rule_index, entry)| {
            if !entry.level.is_enabled() {
                return (Ok(Vec::new()), entry.rule.name(), std::time::Duration::ZERO);
            }

            let rule_start = std::time::Instant::now();
            let mut rule_violations = Vec::new();
            entry
                .rule
                .check_all(ast_context, config, &mut rule_violations);

            for v in &mut rule_violations {
                v.rule_index = rule_index;
                v.level = entry.level;
            }

            if generate_snippets {
                populate_snippets(&mut rule_violations, ast_context);
            }
            let filter_result = filter_violations_in_place(
                &mut rule_violations,
                project_root,
                config,
                entry.rule_config,
            );
            let elapsed = rule_start.elapsed();

            match filter_result {
                Ok(()) => (Ok(rule_violations), entry.rule.name(), elapsed),
                Err(e) => (Err(e), entry.rule.name(), elapsed),
            }
        })
        .collect();

    let mut rule_timings: Vec<(&str, std::time::Duration)> = Vec::new();
    for (rule_violations, name, elapsed) in per_rule {
        if !elapsed.is_zero() {
            rule_timings.push((name, elapsed));
        }
        violations.extend(rule_violations?);
    }

    // Deduplicate: keep only violations from later rules (higher index) when
    // multiple rules fire on same line
    deduplicate_by_rule_precedence(&mut violations);

    // Filter out violations that have inline ignore directives
    violations.retain(|v| {
        !inline_ignore::should_ignore_violation(&v.file, v.line, v.rule, &inline_ignores)
    });

    violations.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
            .then(a.rule.cmp(b.rule))
    });

    Ok((violations, rule_timings))
}

/// List all available rules with their descriptions (text format)
pub fn list_all_rules(config: &Config) {
    let rules = create_all_rules(config);

    let fixable_count = rules.iter().filter(|e| e.rule.fixable()).count();

    println!(
        "{} {}",
        "Available lint rules".bold(),
        format!("({} total, {} auto-fixable)", rules.len(), fixable_count).dimmed()
    );

    for entry in &rules {
        let status = match entry.level {
            RuleLevel::Error => "E".red().bold(),
            RuleLevel::Warn => "W".yellow().bold(),
            RuleLevel::Ignore => "-".dimmed(),
        };
        let name = entry.rule.name().cyan().bold();
        let category = format!("[{}]", entry.rule.category().as_str()).magenta();
        let desc = entry.rule.description().dimmed();
        let fixable = if entry.rule.fixable() {
            format!(" {}", "[auto-fix]".yellow())
        } else {
            "".to_string()
        };
        println!("  {} {} {}{} - {}", status, name, category, fixable, desc);
    }
}

/// List all available rules as JSON
pub fn list_all_rules_json(config: &Config) -> String {
    #[derive(Serialize)]
    struct RuleMetadata {
        name: String,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        long_description: Option<String>,
        category: String,
        fixable: bool,
        opt_in: bool,
        requires_meson: bool,
        min_glib_version: String,
        requires_auto_cleanup: bool,
        config_options: Vec<ConfigOption>,
    }

    #[derive(Serialize)]
    struct RulesOutput {
        rules: Vec<RuleMetadata>,
        total: usize,
        fixable_count: usize,
    }

    let rules = create_all_rules(config);
    let fixable_count = rules.iter().filter(|e| e.rule.fixable()).count();

    let metadata: Vec<RuleMetadata> = rules
        .iter()
        .map(|entry| {
            // Prepend standard config options to rule-specific ones
            let level_default = if entry.opt_in {
                "\"ignore\""
            } else {
                "\"warn\""
            };
            let mut all_options = vec![
                ConfigOption {
                    name: "level",
                    option_type: "string",
                    default_value: level_default,
                    example_value: "\"error\"",
                    description: "Rule severity level: \"error\", \"warn\", or \"ignore\"",
                },
                ConfigOption {
                    name: "ignore",
                    option_type: "array<string>",
                    default_value: "[]",
                    example_value: "[\"tests/**\", \"examples/*.c\"]",
                    description: "Glob patterns for files to ignore for this rule",
                },
            ];
            all_options.extend_from_slice(entry.rule.config_options());

            RuleMetadata {
                name: entry.rule.name().to_string(),
                description: entry.rule.description().to_string(),
                long_description: entry
                    .rule
                    .long_description()
                    .map(std::string::ToString::to_string),
                category: entry.rule.category().as_str().to_string(),
                fixable: entry.rule.fixable(),
                opt_in: entry.opt_in,
                requires_meson: entry.rule.requires_meson(),
                min_glib_version: format!(
                    "{}.{}",
                    entry.min_glib_version.0, entry.min_glib_version.1
                ),
                requires_auto_cleanup: entry.requires_auto_cleanup,
                config_options: all_options,
            }
        })
        .collect();

    let output = RulesOutput {
        total: rules.len(),
        fixable_count,
        rules: metadata,
    };

    serde_json::to_string_pretty(&output).unwrap()
}

/// Keep only the violation with the highest rule_index for each (file, line)
/// pair
fn deduplicate_by_rule_precedence(violations: &mut Vec<Violation>) {
    if violations.len() <= 1 {
        return;
    }

    // Sort by (file, line) so duplicates are adjacent, then by rule_index
    // descending so the best candidate comes first in each group
    violations.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(b.rule_index.cmp(&a.rule_index))
    });

    // Walk linearly: keep the first of each (file, line) group (highest
    // rule_index due to sort order)
    violations.dedup_by(|b, a| a.file == b.file && a.line == b.line);
}
