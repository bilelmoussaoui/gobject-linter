use std::{collections::HashSet, fs, path::Path};

use globset::GlobSetBuilder;
use gobject_linter::{
    ast_context::AstContext, config::Config, fixer, meson::MesonHeaders, rules::Rule,
};

/// Build an AstContext from a single fixture file copied into a temp directory.
/// Also copies any sibling .h files from the fixture directory.
///
/// If `public_headers_file` is provided, its lines are treated as filenames of
/// public installed headers (resolved against the temp dir), faking meson info
/// for rules that require it.
///
/// Returns the TempDir (must stay alive for the duration of the test).
fn build_context_for_file(
    test_file: &Path,
    public_headers_file: Option<&Path>,
) -> (AstContext, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let dest = temp_dir.path().join(test_file.file_name().unwrap());
    fs::copy(test_file, &dest).expect("failed to copy fixture");

    // Also copy any .h files from the same directory (for rules that inspect
    // headers)
    if let Some(fixture_dir) = test_file.parent()
        && let Ok(entries) = fs::read_dir(fixture_dir)
    {
        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "h") {
                let h_dest = temp_dir.path().join(path.file_name().unwrap());
                fs::copy(&path, &h_dest).expect("failed to copy header fixture");
            }
        }
    }

    // Build fake MesonHeaders from the per-test public_headers file. We do
    // this after creating the temp dir so filenames resolve to the right paths.
    let meson_headers = public_headers_file.map(|f| {
        let installed = fs::read_to_string(f)
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|name| temp_dir.path().join(name))
            .collect::<HashSet<_>>();
        MesonHeaders {
            gir: installed.clone(),
            installed,
        }
    });

    let ignore = GlobSetBuilder::new().build().unwrap();
    let ctx = AstContext::build_with_ignore(temp_dir.path(), &ignore, None, meson_headers)
        .expect("failed to build AstContext");

    (ctx, temp_dir)
}

/// Format violations as `filename:line:col: rule: message`, sorted.
fn format_violations(
    violations: &[gobject_linter::rules::Violation],
    strip_prefix: &Path,
) -> String {
    let lines: Vec<String> = violations
        .iter()
        .map(|v| {
            let relative = v.file.strip_prefix(strip_prefix).unwrap_or(&v.file);
            format!(
                "{}:{}:{}: {}: {}",
                relative.display(),
                v.line,
                v.column,
                v.rule,
                v.message
            )
        })
        .collect();
    lines.join("\n")
}

/// Core fixture runner for a single rule.
///
/// - Iterates all `*.c` files in `tests/fixtures/<rule_name>/`
/// - Runs the rule, compares violations against `<stem>.stderr`
/// - If `<stem>.fixed.c` exists, applies fixes and compares the result
/// - If `<stem>.stderr` doesn't exist or `BLESS=1` is set, writes/updates it
fn run_fixture_tests(rule_name: &str, rule: &dyn Rule) {
    let fixtures_dir = Path::new("tests/fixtures").join(rule_name);
    if !fixtures_dir.exists() {
        return;
    }

    let mut test_files: Vec<_> = fs::read_dir(&fixtures_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let path = e.path();
            // Test *.c and standalone *.h files (not already tested as part of .c)
            // Exclude *.fixed.{c,h} (those are expected outputs)
            let ext = path.extension();
            let is_c = ext.is_some_and(|e| e == "c");
            let is_standalone_h = ext.is_some_and(|e| e == "h") && {
                // Only include .h if there's no corresponding .c file
                let stem = path.file_stem().unwrap();
                let c_file = path.with_file_name(stem).with_extension("c");
                !c_file.exists()
            };

            (is_c || is_standalone_h)
                && !path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.ends_with(".fixed"))
        })
        .map(|e| e.path())
        .collect();
    test_files.sort();

    let bless = std::env::var("BLESS").is_ok();
    let mut failures: Vec<String> = Vec::new();

    for test_file in test_files {
        let stem = test_file.file_stem().unwrap().to_str().unwrap().to_owned();
        let ext = test_file.extension().unwrap().to_str().unwrap();
        let stderr_file = fixtures_dir.join(format!("{stem}.stderr"));
        let fixed_file = fixtures_dir.join(format!("{stem}.fixed.{ext}"));

        // --- violation check ---
        // If a <stem>.public_headers file exists, pass it to fake meson info.
        let public_headers_file = fixtures_dir.join(format!("{stem}.public_headers"));
        let public_headers_arg = public_headers_file
            .exists()
            .then_some(public_headers_file.as_path());

        let (ctx, temp_dir) = build_context_for_file(&test_file, public_headers_arg);
        let config = Config::default();

        let mut violations = Vec::new();
        rule.check_all(&ctx, &config, &mut violations);
        violations.sort_by_key(|v| (v.line, v.column));

        let actual_stderr = format_violations(&violations, temp_dir.path());

        if bless || !stderr_file.exists() {
            fs::write(&stderr_file, format!("{actual_stderr}\n")).expect("failed to write .stderr");
            if bless {
                println!("blessed {}", stderr_file.display());
            }
        } else {
            let expected = fs::read_to_string(&stderr_file).unwrap_or_default();
            if actual_stderr.trim() != expected.trim() {
                // Write both to temp files for diff
                let expected_path = temp_dir.path().join("expected.stderr");
                let actual_path = temp_dir.path().join("actual.stderr");
                fs::write(&expected_path, &expected).expect("failed to write expected");
                fs::write(&actual_path, &actual_stderr).expect("failed to write actual");

                // Run diff to show the differences
                let diff_output = std::process::Command::new("diff")
                    .arg("-u")
                    .arg("--label")
                    .arg("expected")
                    .arg("--label")
                    .arg("actual")
                    .arg(&expected_path)
                    .arg(&actual_path)
                    .output()
                    .map_or_else(
                        |_| {
                            format!(
                                "Failed to run diff\n--- expected ---\n{}\n--- got ---\n{}",
                                expected.trim(),
                                actual_stderr.trim()
                            )
                        },
                        |o| String::from_utf8_lossy(&o.stdout).to_string(),
                    );

                failures.push(format!(
                    "fixture {rule_name}/{stem}: violations mismatch\n{}",
                    diff_output
                ));
            }
        }

        // --- fix check ---
        if fixed_file.exists() {
            fixer::apply_fixes(&violations).expect("failed to apply fixes");

            let temp_c = temp_dir.path().join(test_file.file_name().unwrap());
            let actual_fixed = fs::read_to_string(&temp_c).expect("failed to read fixed file");
            let expected_fixed = fs::read_to_string(&fixed_file).expect("failed to read .fixed.c");

            if actual_fixed != expected_fixed {
                // Write both to temp files for diff
                let expected_path = temp_dir.path().join("expected.c");
                let actual_path = temp_dir.path().join("actual.c");
                fs::write(&expected_path, &expected_fixed).expect("failed to write expected");
                fs::write(&actual_path, &actual_fixed).expect("failed to write actual");

                // Run diff to show the differences
                let diff_output = std::process::Command::new("diff")
                    .arg("-u")
                    .arg("--label")
                    .arg("expected")
                    .arg("--label")
                    .arg("actual")
                    .arg(&expected_path)
                    .arg(&actual_path)
                    .output()
                    .map_or_else(
                        |_| {
                            format!(
                                "Failed to run diff\n--- expected ---\n{}\n--- got ---\n{}",
                                expected_fixed.trim(),
                                actual_fixed.trim()
                            )
                        },
                        |o| String::from_utf8_lossy(&o.stdout).to_string(),
                    );

                failures.push(format!(
                    "fixture {rule_name}/{stem}: fix output mismatch\n{}",
                    diff_output
                ));
            }

            // --- post-fix violation check ---
            // Re-run the rule on the fixed file to verify which violations remain.
            let fixed_stderr_file = fixtures_dir.join(format!("{stem}.fixed.stderr"));
            let (ctx_fixed, temp_dir_fixed) = build_context_for_file(&temp_c, public_headers_arg);
            let mut post_fix_violations = Vec::new();
            rule.check_all(&ctx_fixed, &config, &mut post_fix_violations);
            post_fix_violations.sort_by_key(|v| (v.line, v.column));
            let actual_fixed_stderr =
                format_violations(&post_fix_violations, temp_dir_fixed.path());

            if bless || !fixed_stderr_file.exists() {
                fs::write(&fixed_stderr_file, format!("{actual_fixed_stderr}\n"))
                    .expect("failed to write .fixed.stderr");
                if bless {
                    println!("blessed {}", fixed_stderr_file.display());
                }
            } else {
                let expected = fs::read_to_string(&fixed_stderr_file).unwrap_or_default();
                if actual_fixed_stderr.trim() != expected.trim() {
                    // Write both to temp files for diff
                    let expected_path = temp_dir_fixed.path().join("expected.stderr");
                    let actual_path = temp_dir_fixed.path().join("actual.stderr");
                    fs::write(&expected_path, &expected).expect("failed to write expected");
                    fs::write(&actual_path, &actual_fixed_stderr).expect("failed to write actual");

                    // Run diff to show the differences
                    let diff_output = std::process::Command::new("diff")
                        .arg("-u")
                        .arg("--label")
                        .arg("expected")
                        .arg("--label")
                        .arg("actual")
                        .arg(&expected_path)
                        .arg(&actual_path)
                        .output()
                        .map_or_else(
                            |_| {
                                format!(
                                    "Failed to run diff\n--- expected ---\n{}\n--- got ---\n{}",
                                    expected.trim(),
                                    actual_fixed_stderr.trim()
                                )
                            },
                            |o| String::from_utf8_lossy(&o.stdout).to_string(),
                        );

                    failures.push(format!(
                        "fixture {rule_name}/{stem}: post-fix violations mismatch\n{}",
                        diff_output
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!("\n{}", failures.join("\n\n"));
    }
}

macro_rules! rule_test {
    ($rule_name:ident, $rule:expr) => {
        #[test]
        fn $rule_name() {
            run_fixture_tests(stringify!($rule_name), &$rule);
        }
    };
}

rule_test!(
    deprecated_add_private,
    gobject_linter::rules::DeprecatedAddPrivate
);
rule_test!(g_error_init, gobject_linter::rules::GErrorInit);
rule_test!(g_error_leak, gobject_linter::rules::GErrorLeak);
rule_test!(
    g_source_id_not_stored,
    gobject_linter::rules::GSourceIdNotStored
);
rule_test!(
    g_object_virtual_methods_chain_up,
    gobject_linter::rules::GObjectVirtualMethodsChainUp
);
rule_test!(
    g_param_spec_null_nick_blurb,
    gobject_linter::rules::GParamSpecNullNickBlurb
);
rule_test!(
    property_canonical_name,
    gobject_linter::rules::PropertyCanonicalName
);
rule_test!(
    g_param_spec_static_strings,
    gobject_linter::rules::GParamSpecStaticStrings
);
rule_test!(g_task_source_tag, gobject_linter::rules::GTaskSourceTag);
rule_test!(include_order, gobject_linter::rules::IncludeOrder);
rule_test!(
    inconsistent_function_signature,
    gobject_linter::rules::InconsistentFunctionSignature
);
rule_test!(
    matching_declare_define,
    gobject_linter::rules::MatchingDeclareDefine
);
rule_test!(
    missing_autoptr_cleanup,
    gobject_linter::rules::MissingAutoptrCleanup
);
rule_test!(
    missing_implementation,
    gobject_linter::rules::MissingImplementation
);
rule_test!(no_g_auto_macros, gobject_linter::rules::NoGAutoMacros);
rule_test!(
    property_enum_convention,
    gobject_linter::rules::PropertyEnumConvention
);
rule_test!(
    property_enum_coverage,
    gobject_linter::rules::PropertyEnumCoverage
);
rule_test!(
    property_switch_exhaustiveness,
    gobject_linter::rules::PropertySwitchExhaustiveness
);
rule_test!(
    signal_canonical_name,
    gobject_linter::rules::SignalCanonicalName
);
rule_test!(
    signal_enum_coverage,
    gobject_linter::rules::SignalEnumCoverage
);
rule_test!(
    use_g_object_new_with_properties,
    gobject_linter::rules::UseGObjectNewWithProperties
);
rule_test!(
    use_g_bytes_unref_to_data,
    gobject_linter::rules::UseGBytesUnrefToData
);
rule_test!(use_auto_cleanup, gobject_linter::rules::UseAutoCleanup);
rule_test!(
    use_g_file_load_bytes,
    gobject_linter::rules::UseGFileLoadBytes
);
rule_test!(
    use_g_gnuc_flag_enum,
    gobject_linter::rules::UseGGnucFlagEnum
);
rule_test!(use_g_new, gobject_linter::rules::UseGNew);
rule_test!(
    use_g_object_class_install_properties,
    gobject_linter::rules::UseGObjectClassInstallProperties
);
rule_test!(use_g_source_once, gobject_linter::rules::UseGSourceOnce);
rule_test!(
    unnecessary_null_check,
    gobject_linter::rules::UnnecessaryNullCheck
);
rule_test!(
    use_clear_functions,
    gobject_linter::rules::UseClearFunctions
);
rule_test!(
    use_explicit_default_flags,
    gobject_linter::rules::UseExplicitDefaultFlags
);
rule_test!(
    use_g_object_notify_by_pspec,
    gobject_linter::rules::UseGObjectNotifyByPspec
);
rule_test!(use_g_set_object, gobject_linter::rules::UseGSetObject);
rule_test!(use_g_set_str, gobject_linter::rules::UseGSetStr);
rule_test!(
    use_g_settings_typed,
    gobject_linter::rules::UseGSettingsTyped
);
rule_test!(
    use_g_source_constants,
    gobject_linter::rules::UseGSourceConstants
);
rule_test!(use_g_steal_pointer, gobject_linter::rules::UseGStealPointer);
rule_test!(
    use_g_str_has_prefix_suffix,
    gobject_linter::rules::UseGStrHasPrefixSuffix
);
rule_test!(
    use_g_ascii_functions,
    gobject_linter::rules::UseGAsciiFunctions
);
rule_test!(use_g_strlcpy, gobject_linter::rules::UseGStrlcpy);
rule_test!(
    strcmp_explicit_comparison,
    gobject_linter::rules::StrcmpExplicitComparison
);
rule_test!(use_g_strcmp0, gobject_linter::rules::UseGStrcmp0);
rule_test!(
    use_g_string_free_and_steal,
    gobject_linter::rules::UseGStringFreeAndSteal
);
rule_test!(
    use_g_value_set_static_string,
    gobject_linter::rules::UseGValueSetStaticString
);
rule_test!(
    use_g_variant_new_typed,
    gobject_linter::rules::UseGVariantNewTyped
);
rule_test!(
    untranslated_string,
    gobject_linter::rules::UntranslatedString
);
rule_test!(use_pragma_once, gobject_linter::rules::UsePragmaOnce);
rule_test!(dead_code, gobject_linter::rules::DeadCode);
rule_test!(
    missing_export_macro,
    gobject_linter::rules::MissingExportMacro
);
rule_test!(type_style, gobject_linter::rules::TypeStyle);
rule_test!(gi_missing_since, gobject_linter::rules::GiMissingSince);
rule_test!(
    gi_not_bindings_friendly,
    gobject_linter::rules::GiNotBindingsFriendly
);
