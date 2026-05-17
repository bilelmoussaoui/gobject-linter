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
}

/// Macro to define all rules in execution order.
/// Format: (config_field, RuleType)
#[macro_export]
macro_rules! for_each_rule {
    ($callback:ident) => {
        $callback! {
            (dead_code, DeadCode),
            (unused_vfunc, UnusedVfunc),
            (include_order, IncludeOrder),
            (inconsistent_function_signature, InconsistentFunctionSignature),
            (use_pragma_once, UsePragmaOnce),
            (missing_implementation, MissingImplementation),
            (missing_autoptr_cleanup, MissingAutoptrCleanup),
            (missing_g_begin_decls, MissingGBeginDecls),
            (missing_export_macro, MissingExportMacro),
            (no_g_auto_macros, NoGAutoMacros),
            (deprecated_add_private, DeprecatedAddPrivate),
            (matching_declare_define, MatchingDeclareDefine),
            (use_g_new, UseGNew),
            (use_g_object_class_install_properties, UseGObjectClassInstallProperties),
            (use_g_settings_typed, UseGSettingsTyped),
            (use_g_value_set_static_string, UseGValueSetStaticString),
            (use_g_variant_new_typed, UseGVariantNewTyped),
            (strcmp_explicit_comparison, StrcmpExplicitComparison),
            (type_style, TypeStyle),
            (use_g_strcmp0, UseGStrcmp0),
            (use_clear_functions, UseClearFunctions),
            (use_explicit_default_flags, UseExplicitDefaultFlags),
            (g_param_spec_null_nick_blurb, GParamSpecNullNickBlurb),
            (g_param_spec_static_strings, GParamSpecStaticStrings),
            (property_canonical_name, PropertyCanonicalName),
            (g_error_init, GErrorInit),
            (g_error_leak, GErrorLeak),
            (g_source_id_not_stored, GSourceIdNotStored),
            (property_enum_convention, PropertyEnumConvention),
            (property_enum_coverage, PropertyEnumCoverage),
            (property_switch_exhaustiveness, PropertySwitchExhaustiveness),
            (signal_canonical_name, SignalCanonicalName),
            (signal_enum_coverage, SignalEnumCoverage),
            (g_object_virtual_methods_chain_up, GObjectVirtualMethodsChainUp),
            (g_task_source_tag, GTaskSourceTag),
            (unnecessary_null_check, UnnecessaryNullCheck),
            (use_g_set_object, UseGSetObject),
            (use_g_set_str, UseGSetStr),
            (use_auto_cleanup, UseAutoCleanup),
            (use_g_bytes_unref_to_data, UseGBytesUnrefToData),
            (use_g_file_load_bytes, UseGFileLoadBytes),
            (use_g_gnuc_flag_enum, UseGGnucFlagEnum),
            (use_g_object_new_with_properties, UseGObjectNewWithProperties),
            (use_g_object_notify_by_pspec, UseGObjectNotifyByPspec),
            (use_g_string_free_and_steal, UseGStringFreeAndSteal),
            (use_g_source_once, UseGSourceOnce),
            (use_g_source_constants, UseGSourceConstants),
            (use_g_steal_pointer, UseGStealPointer),
            (use_g_str_has_prefix_suffix, UseGStrHasPrefixSuffix),
            (use_g_ascii_functions, UseGAsciiFunctions),
            (use_g_strlcpy, UseGStrlcpy),
            (untranslated_string, UntranslatedString),
            (gi_missing_since, GiMissingSince),
            (gi_not_bindings_friendly, GiNotBindingsFriendly),
        }
    };
}

macro_rules! impl_create_all_rules {
    ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
        /// Create all rule instances in execution order
        pub fn create_all_rules<'a>(config: &'a Config) -> Vec<RuleEntry<'a>> {
            vec![
                $({
                    let rule = $rule_type;
                    let min_ver = rule.min_glib_version();
                    let opt_in = rule.opt_in();
                    let requires_auto_cleanup = rule.requires_auto_cleanup();
                    let level = if is_rule_compatible(config, min_ver) {
                        let default_level = if opt_in {
                            RuleLevel::Ignore
                        } else {
                            config.default_level.unwrap_or(RuleLevel::Warn)
                        };
                        let configured = config.rules.$config_field.level.unwrap_or(default_level);
                        apply_msvc_compatibility(config, stringify!($config_field), requires_auto_cleanup, configured)
                    } else {
                        RuleLevel::Ignore
                    };
                    RuleEntry {
                        rule: Box::new(rule),
                        level,
                        rule_config: &config.rules.$config_field,
                    }
                },)*
            ]
        }
    };
}

/// Check if a rule is compatible with the configured minimum GLib version
fn is_rule_compatible(config: &Config, required: Option<(u32, u32)>) -> bool {
    let Some((req_major, req_minor)) = required else {
        return true;
    };
    if let Some((major, minor)) = config.min_glib_version {
        major > req_major || (major == req_major && minor >= req_minor)
    } else {
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

macro_rules! impl_validate_config {
    ($(($config_field:ident, $rule_type:ident)),* $(,)?) => {
        /// Check that explicitly enabled rules are compatible with config
        /// constraints (min_glib_version, msvc_compatible). Returns an error
        /// describing the first conflict found.
        pub fn validate_config(config: &Config) -> Result<()> {
            $(
            {
                let rule_config = &config.rules.$config_field;
                if let Some(level) = rule_config.level
                    && level.is_enabled()
                {
                    let rule = $rule_type;
                    let name = stringify!($config_field);

                    if let Some((req_major, req_minor)) = rule.min_glib_version()
                        && !is_rule_compatible(config, Some((req_major, req_minor)))
                    {
                        let (cfg_major, cfg_minor) = config.min_glib_version.unwrap_or((2, 0));
                        anyhow::bail!(
                            "Rule '{}' requires GLib >= {}.{}, but min_glib_version is {}.{}",
                            name, req_major, req_minor, cfg_major, cfg_minor,
                        );
                    }

                    if config.msvc_compatible && rule.requires_auto_cleanup() {
                        anyhow::bail!(
                            "Rule '{}' requires g_auto* macros, which are unavailable with msvc_compatible = true",
                            name,
                        );
                    }

                    if name == "no_g_auto_macros" && !config.msvc_compatible {
                        anyhow::bail!(
                            "Rule 'no_g_auto_macros' is only meaningful with msvc_compatible = true",
                        );
                    }
                }
            }
            )*
            Ok(())
        }
    };
}

for_each_rule!(impl_validate_config);

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
        let category = format!("[{}]", entry.rule.category()).magenta();
        let desc = entry.rule.description().dimmed();

        let mut tags = Vec::new();
        if entry.rule.fixable() {
            tags.push("[auto-fix]".yellow().to_string());
        }
        if entry.rule.opt_in() {
            tags.push("[opt-in]".blue().to_string());
        }
        if entry.rule.requires_meson() {
            tags.push("[meson]".blue().to_string());
        }
        if entry.rule.requires_auto_cleanup() {
            tags.push("[no-msvc]".blue().to_string());
        }
        if let Some((major, minor)) = entry.rule.min_glib_version()
            && (major > 2 || (major == 2 && minor > 0))
        {
            tags.push(format!("[glib>={major}.{minor}]").dimmed().to_string());
        }
        let tags_str = if tags.is_empty() {
            String::new()
        } else {
            format!(" {}", tags.join(" "))
        };
        println!("  {} {} {}{} - {}", status, name, category, tags_str, desc);
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
        #[serde(skip_serializing_if = "Option::is_none")]
        opt_in_reason: Option<String>,
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
            let level_default = if entry.rule.opt_in() {
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
                opt_in: entry.rule.opt_in(),
                opt_in_reason: entry.rule.opt_in_reason().map(String::from),
                requires_meson: entry.rule.requires_meson(),
                min_glib_version: entry
                    .rule
                    .min_glib_version()
                    .map_or_else(|| "2.0".to_string(), |(maj, min)| format!("{maj}.{min}")),
                requires_auto_cleanup: entry.rule.requires_auto_cleanup(),
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

/// Keep only the violation with the highest rule_index for each (file, line,
/// column) position
fn deduplicate_by_rule_precedence(violations: &mut Vec<Violation>) {
    if violations.len() <= 1 {
        return;
    }

    // Sort by (file, line, column) so duplicates are adjacent, then by
    // rule_index descending so the best candidate comes first in each group
    violations.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
            .then(b.rule_index.cmp(&a.rule_index))
    });

    // Walk linearly: keep the first of each (file, line, column) group
    // (highest rule_index due to sort order)
    violations.dedup_by(|b, a| a.file == b.file && a.line == b.line && a.column == b.column);
}
