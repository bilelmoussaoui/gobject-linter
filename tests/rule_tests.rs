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

    let ctx = build_ast_context(&temp_dir, public_headers_file);
    (ctx, temp_dir)
}

/// Build an AstContext from a directory of fixture files copied into a temp
/// dir.
fn build_context_for_dir(
    fixture_dir: &Path,
    public_headers_file: Option<&Path>,
) -> (AstContext, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");

    for entry in fs::read_dir(fixture_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();
        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str());
            if matches!(ext, Some("c" | "h")) {
                let dest = temp_dir.path().join(path.file_name().unwrap());
                fs::copy(&path, &dest).expect("failed to copy fixture file");
            }
        }
    }

    let ctx = build_ast_context(&temp_dir, public_headers_file);
    (ctx, temp_dir)
}

fn build_ast_context(
    temp_dir: &tempfile::TempDir,
    public_headers_file: Option<&Path>,
) -> AstContext {
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
    AstContext::build_with_ignore(temp_dir.path(), &ignore, None, meson_headers)
        .expect("failed to build AstContext")
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

fn check_violations(
    rule_name: &str,
    case_name: &str,
    stderr_file: &Path,
    violations: &[gobject_linter::rules::Violation],
    strip_prefix: &Path,
    bless: bool,
    failures: &mut Vec<String>,
) {
    let actual_stderr = format_violations(violations, strip_prefix);

    if bless || !stderr_file.exists() {
        fs::write(stderr_file, format!("{actual_stderr}\n")).expect("failed to write .stderr");
        if bless {
            println!("blessed {}", stderr_file.display());
        }
    } else {
        let expected = fs::read_to_string(stderr_file).unwrap_or_default();
        if actual_stderr.trim() != expected.trim() {
            let tmp = tempfile::tempdir().unwrap();
            let expected_path = tmp.path().join("expected.stderr");
            let actual_path = tmp.path().join("actual.stderr");
            fs::write(&expected_path, &expected).expect("failed to write expected");
            fs::write(&actual_path, &actual_stderr).expect("failed to write actual");

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
                "fixture {rule_name}/{case_name}: violations mismatch\n{}",
                diff_output
            ));
        }
    }
}

/// Core fixture runner for a single rule.
///
/// Supports two fixture layouts:
/// 1. Flat files: `tests/fixtures/<rule>/foo.c` + `foo.stderr`
/// 2. Subdirectories: `tests/fixtures/<rule>/case_name/` containing `.c`/`.h`
///    files and `expected.stderr`
fn run_fixture_tests(rule_name: &str, rule: &dyn Rule) {
    let fixtures_dir = Path::new("tests/fixtures").join(rule_name);
    if !fixtures_dir.exists() {
        return;
    }

    let bless = std::env::var("BLESS").is_ok();
    let mut failures: Vec<String> = Vec::new();

    let mut test_files: Vec<_> = fs::read_dir(&fixtures_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let path = e.path();
            if path.is_dir() {
                return false;
            }
            let ext = path.extension();
            let is_c = ext.is_some_and(|e| e == "c");
            let is_standalone_h = ext.is_some_and(|e| e == "h") && {
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

    for test_file in &test_files {
        let stem = test_file.file_stem().unwrap().to_str().unwrap().to_owned();
        let ext = test_file.extension().unwrap().to_str().unwrap();
        let stderr_file = fixtures_dir.join(format!("{stem}.stderr"));
        let fixed_file = fixtures_dir.join(format!("{stem}.fixed.{ext}"));

        let public_headers_file = fixtures_dir.join(format!("{stem}.public_headers"));
        let public_headers_arg = public_headers_file
            .exists()
            .then_some(public_headers_file.as_path());

        let (ctx, temp_dir) = build_context_for_file(test_file, public_headers_arg);
        let config = Config::default();

        let mut violations = Vec::new();
        rule.check_all(&ctx, &config, &mut violations);
        violations.sort_by_key(|v| (v.line, v.column));

        check_violations(
            rule_name,
            &stem,
            &stderr_file,
            &violations,
            temp_dir.path(),
            bless,
            &mut failures,
        );

        // --- fix check ---
        if fixed_file.exists() {
            fixer::apply_fixes(&violations).expect("failed to apply fixes");

            let temp_c = temp_dir.path().join(test_file.file_name().unwrap());
            let actual_fixed = fs::read_to_string(&temp_c).expect("failed to read fixed file");
            let expected_fixed = fs::read_to_string(&fixed_file).expect("failed to read .fixed.c");

            if actual_fixed != expected_fixed {
                let tmp = tempfile::tempdir().unwrap();
                let expected_path = tmp.path().join("expected.c");
                let actual_path = tmp.path().join("actual.c");
                fs::write(&expected_path, &expected_fixed).expect("failed to write expected");
                fs::write(&actual_path, &actual_fixed).expect("failed to write actual");

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

            let fixed_stderr_file = fixtures_dir.join(format!("{stem}.fixed.stderr"));
            let (ctx_fixed, temp_dir_fixed) = build_context_for_file(&temp_c, public_headers_arg);
            let mut post_fix_violations = Vec::new();
            rule.check_all(&ctx_fixed, &config, &mut post_fix_violations);
            post_fix_violations.sort_by_key(|v| (v.line, v.column));

            check_violations(
                rule_name,
                &format!("{stem}.fixed"),
                &fixed_stderr_file,
                &post_fix_violations,
                temp_dir_fixed.path(),
                bless,
                &mut failures,
            );
        }
    }

    let mut subdirs: Vec<_> = fs::read_dir(&fixtures_dir)
        .unwrap()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    subdirs.sort();

    for subdir in &subdirs {
        let case_name = subdir.file_name().unwrap().to_str().unwrap().to_owned();
        let stderr_file = subdir.join("expected.stderr");

        let public_headers_file = subdir.join("public_headers");
        let public_headers_arg = public_headers_file
            .exists()
            .then_some(public_headers_file.as_path());

        let (ctx, temp_dir) = build_context_for_dir(subdir, public_headers_arg);
        let config = Config::default();

        let mut violations = Vec::new();
        rule.check_all(&ctx, &config, &mut violations);
        violations.sort_by_key(|v| (v.line, v.column));

        check_violations(
            rule_name,
            &case_name,
            &stderr_file,
            &violations,
            temp_dir.path(),
            bless,
            &mut failures,
        );
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
    missing_g_begin_decls,
    gobject_linter::rules::MissingGBeginDecls
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
