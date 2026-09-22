use std::path::{Path, PathBuf};
use std::time::Duration;

use gobject_linter::config::{Config, RuleLevel};
use gobject_linter::output::{gcc, gitlab_codequality, sarif};
use gobject_linter::reporter;
use gobject_linter::rules::{Category, Fix, Violation};

fn make_violation(
    file: &str,
    line: usize,
    column: usize,
    message: &str,
    rule: &'static str,
    category: Category,
    level: RuleLevel,
) -> Violation {
    Violation {
        file: PathBuf::from(file),
        line,
        column,
        message: message.to_string(),
        rule,
        category,
        level,
        snippet: None,
        rule_index: 0,
        fixes: vec![],
    }
}

// ---------------------------------------------------------------------------
// reporter::format_duration
// ---------------------------------------------------------------------------

#[test]
fn format_duration_sub_second() {
    let d = Duration::from_millis(42);
    assert_eq!(reporter::format_duration(d), "42ms");
}

#[test]
fn format_duration_zero() {
    let d = Duration::from_millis(0);
    assert_eq!(reporter::format_duration(d), "0ms");
}

#[test]
fn format_duration_one_second() {
    let d = Duration::from_secs(1);
    assert_eq!(reporter::format_duration(d), "1.00s");
}

#[test]
fn format_duration_over_second() {
    let d = Duration::from_millis(2500);
    assert_eq!(reporter::format_duration(d), "2.50s");
}

#[test]
fn format_duration_exactly_999ms() {
    let d = Duration::from_millis(999);
    assert_eq!(reporter::format_duration(d), "999ms");
}

// ---------------------------------------------------------------------------
// output::gitlab_codequality
// ---------------------------------------------------------------------------

#[test]
fn gitlab_empty_violations() {
    let output = gitlab_codequality::generate_gitlab_codequality(&[], Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed, serde_json::json!([]));
}

#[test]
fn gitlab_single_error() {
    let v = make_violation(
        "/project/src/test.c",
        10,
        5,
        "missing prototype",
        "missing_implementation",
        Category::Correctness,
        RuleLevel::Error,
    );
    let output = gitlab_codequality::generate_gitlab_codequality(&[v], Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let issues = parsed.as_array().unwrap();
    assert_eq!(issues.len(), 1);

    let issue = &issues[0];
    assert_eq!(issue["description"], "missing prototype");
    assert_eq!(issue["check_name"], "missing_implementation");
    assert_eq!(issue["severity"], "blocker");
    assert_eq!(issue["location"]["path"], "src/test.c");
    assert_eq!(issue["location"]["positions"]["begin"]["line"], 10);
    assert_eq!(issue["location"]["positions"]["begin"]["column"], 5);

    // fingerprint must be a string
    assert!(issue["fingerprint"].is_string());
}

#[test]
fn gitlab_severity_mapping_warn_correctness() {
    let v = make_violation(
        "/project/test.c",
        1,
        1,
        "msg",
        "rule",
        Category::Correctness,
        RuleLevel::Warn,
    );
    let output = gitlab_codequality::generate_gitlab_codequality(&[v], Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed[0]["severity"], "critical");
}

#[test]
fn gitlab_severity_mapping_warn_perf() {
    let v = make_violation(
        "/project/test.c",
        1,
        1,
        "msg",
        "rule",
        Category::Perf,
        RuleLevel::Warn,
    );
    let output = gitlab_codequality::generate_gitlab_codequality(&[v], Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed[0]["severity"], "critical");
}

#[test]
fn gitlab_severity_mapping_warn_style() {
    let v = make_violation(
        "/project/test.c",
        1,
        1,
        "msg",
        "rule",
        Category::Style,
        RuleLevel::Warn,
    );
    let output = gitlab_codequality::generate_gitlab_codequality(&[v], Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed[0]["severity"], "info");
}

#[test]
fn gitlab_multiple_violations() {
    let violations = vec![
        make_violation(
            "/project/a.c",
            1,
            1,
            "first",
            "rule_a",
            Category::Correctness,
            RuleLevel::Error,
        ),
        make_violation(
            "/project/b.c",
            20,
            3,
            "second",
            "rule_b",
            Category::Style,
            RuleLevel::Warn,
        ),
    ];
    let output =
        gitlab_codequality::generate_gitlab_codequality(&violations, Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let issues = parsed.as_array().unwrap();
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0]["description"], "first");
    assert_eq!(issues[1]["description"], "second");

    // Fingerprints should differ
    assert_ne!(issues[0]["fingerprint"], issues[1]["fingerprint"]);
}

#[test]
fn gitlab_relative_path_stripping() {
    let v = make_violation(
        "/my/project/src/deep/file.c",
        5,
        10,
        "msg",
        "rule",
        Category::Style,
        RuleLevel::Warn,
    );
    let output = gitlab_codequality::generate_gitlab_codequality(&[v], Path::new("/my/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed[0]["location"]["path"], "src/deep/file.c");
}

// ---------------------------------------------------------------------------
// output::sarif
// ---------------------------------------------------------------------------

#[test]
fn sarif_empty_violations() {
    let config = Config::default();
    let output = sarif::generate_sarif(&[], &config, Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["version"], "2.1.0");
    assert!(parsed["$schema"].as_str().unwrap().contains("sarif"));

    let runs = parsed["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);

    let driver = &runs[0]["tool"]["driver"];
    assert_eq!(driver["name"], "gobject-linter");
    assert!(driver["version"].is_string());
    assert!(driver["rules"].is_array());

    let results = runs[0]["results"].as_array().unwrap();
    assert!(results.is_empty());
}

#[test]
fn sarif_single_violation() {
    let config = Config::default();
    let v = make_violation(
        "/project/src/widget.c",
        42,
        8,
        "Use g_new instead of malloc",
        "use_g_new",
        Category::Style,
        RuleLevel::Warn,
    );
    let output = sarif::generate_sarif(&[v], &config, Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let results = parsed["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);

    let result = &results[0];
    assert_eq!(result["ruleId"], "use_g_new");
    assert_eq!(result["level"], "warning");
    assert_eq!(result["message"]["text"], "Use g_new instead of malloc");

    let location = &result["locations"][0]["physicalLocation"];
    assert_eq!(location["artifactLocation"]["uri"], "src/widget.c");
    assert_eq!(location["region"]["startLine"], 42);
    assert_eq!(location["region"]["startColumn"], 8);
}

#[test]
fn sarif_error_level_mapping() {
    let config = Config::default();
    let v = make_violation(
        "/project/test.c",
        1,
        1,
        "msg",
        "test_rule",
        Category::Correctness,
        RuleLevel::Error,
    );
    let output = sarif::generate_sarif(&[v], &config, Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(parsed["runs"][0]["results"][0]["level"], "error");
}

#[test]
fn sarif_with_fix() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("fix_test.c");
    // "hello world!" - fix replaces bytes 5..10 with "replacement"
    std::fs::write(&test_file, b"hello world!\n").unwrap();

    let mut v = make_violation(
        &test_file.display().to_string(),
        1,
        6,
        "replace world",
        "test_rule",
        Category::Style,
        RuleLevel::Warn,
    );
    v.fixes = vec![Fix::new(5, 10, "earth")];

    let config = Config::default();
    let output = sarif::generate_sarif(&[v], &config, temp_dir.path());
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    let fixes = result["fixes"].as_array().unwrap();
    assert_eq!(fixes.len(), 1);

    let replacements = &fixes[0]["artifactChanges"][0]["replacements"];
    let repl = &replacements[0];
    // Byte 5 in "hello world!" is column 6 on line 1
    assert_eq!(repl["deletedRegion"]["startLine"], 1);
    assert_eq!(repl["deletedRegion"]["startColumn"], 6);
    assert_eq!(repl["insertedContent"]["text"], "earth");
}

#[test]
fn sarif_with_deletion_fix() {
    let temp_dir = tempfile::tempdir().unwrap();
    let test_file = temp_dir.path().join("del_test.c");
    std::fs::write(&test_file, b"abcdef\n").unwrap();

    let mut v = make_violation(
        &test_file.display().to_string(),
        1,
        3,
        "remove cd",
        "test_rule",
        Category::Style,
        RuleLevel::Warn,
    );
    v.fixes = vec![Fix::delete(2, 4)];

    let config = Config::default();
    let output = sarif::generate_sarif(&[v], &config, temp_dir.path());
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    let repl = &result["fixes"][0]["artifactChanges"][0]["replacements"][0];
    // No insertedContent for deletion
    assert!(repl.get("insertedContent").is_none());
    assert_eq!(repl["deletedRegion"]["startLine"], 1);
    assert_eq!(repl["deletedRegion"]["startColumn"], 3);
    assert_eq!(repl["deletedRegion"]["endLine"], 1);
    assert_eq!(repl["deletedRegion"]["endColumn"], 5);
}

#[test]
fn sarif_with_editor_url() {
    let config = Config {
        editor_url: Some("vscode://file/{path}:{line}:{column}".to_string()),
        ..Config::default()
    };

    let v = make_violation(
        "/project/src/test.c",
        15,
        3,
        "msg",
        "test_rule",
        Category::Style,
        RuleLevel::Warn,
    );
    let output = sarif::generate_sarif(&[v], &config, Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let result = &parsed["runs"][0]["results"][0];
    let hovers = result["hovers"].as_array().unwrap();
    assert_eq!(hovers.len(), 1);
    let text = hovers[0]["text"].as_str().unwrap();
    assert!(text.contains("vscode://file//project/src/test.c:15:3"));
}

#[test]
fn sarif_rules_metadata_present() {
    let config = Config::default();
    let output = sarif::generate_sarif(&[], &config, Path::new("/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let rules = parsed["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    // Should have many rules defined
    assert!(rules.len() > 10);

    // Each rule should have required fields
    let first = &rules[0];
    assert!(first["id"].is_string());
    assert!(first["shortDescription"]["text"].is_string());
    assert!(first["defaultConfiguration"]["level"].is_string());
    assert!(first["properties"]["category"].is_string());
}

#[test]
fn sarif_relative_path_for_violations() {
    let config = Config::default();
    let v = make_violation(
        "/my/project/deeply/nested/file.c",
        1,
        1,
        "msg",
        "test_rule",
        Category::Correctness,
        RuleLevel::Error,
    );
    let output = sarif::generate_sarif(&[v], &config, Path::new("/my/project"));
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

    let uri = parsed["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
        ["artifactLocation"]["uri"]
        .as_str()
        .unwrap();
    assert_eq!(uri, "deeply/nested/file.c");
}

// ---------------------------------------------------------------------------
// output::gcc (prints to stdout — we test via process)
// ---------------------------------------------------------------------------

#[test]
fn gcc_empty_violations() {
    // generate_gcc with empty violations prints "No violations found" to stderr
    // and nothing to stdout. We can't easily capture these here, but we can
    // at least confirm it doesn't panic.
    gcc::generate_gcc(&[]);
}

#[test]
fn gcc_single_error_no_panic() {
    let v = make_violation(
        "/project/src/test.c",
        10,
        5,
        "bad code",
        "test_rule",
        Category::Correctness,
        RuleLevel::Error,
    );
    gcc::generate_gcc(&[v]);
}

#[test]
fn gcc_single_warning_no_panic() {
    let v = make_violation(
        "/project/src/test.c",
        20,
        1,
        "could be better",
        "style_rule",
        Category::Style,
        RuleLevel::Warn,
    );
    gcc::generate_gcc(&[v]);
}

#[test]
fn gcc_mixed_levels_no_panic() {
    let violations = vec![
        make_violation(
            "/project/a.c",
            1,
            1,
            "error msg",
            "rule_a",
            Category::Correctness,
            RuleLevel::Error,
        ),
        make_violation(
            "/project/b.c",
            2,
            1,
            "warning msg",
            "rule_b",
            Category::Style,
            RuleLevel::Warn,
        ),
    ];
    gcc::generate_gcc(&violations);
}
