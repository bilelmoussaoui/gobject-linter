use std::{fs, path::PathBuf};

use gobject_linter::{
    config::RuleLevel,
    fixer,
    rules::{Category, Fix, Violation},
};

fn make_violation(file: PathBuf, fixes: Vec<Fix>) -> Violation {
    Violation {
        file,
        line: 1,
        column: 1,
        message: "test".to_string(),
        rule: "test_rule",
        category: Category::Correctness,
        level: RuleLevel::Warn,
        snippet: None,
        rule_index: 0,
        fixes,
    }
}

#[test]
fn apply_single_replacement() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.c");
    fs::write(&file, "hello world").unwrap();

    let violations = vec![make_violation(
        file.clone(),
        vec![Fix::new(0, 5, "goodbye")],
    )];

    let count = fixer::apply_fixes(&violations).unwrap();
    assert_eq!(count, 1);
    assert_eq!(fs::read_to_string(&file).unwrap(), "goodbye world");
}

#[test]
fn apply_deletion() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.c");
    fs::write(&file, "hello world").unwrap();

    let violations = vec![make_violation(file.clone(), vec![Fix::delete(5, 11)])];

    let count = fixer::apply_fixes(&violations).unwrap();
    assert_eq!(count, 1);
    assert_eq!(fs::read_to_string(&file).unwrap(), "hello");
}

#[test]
fn apply_multiple_non_overlapping_fixes_from_different_violations() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.c");
    fs::write(&file, "aaa bbb ccc").unwrap();

    let violations = vec![
        make_violation(file.clone(), vec![Fix::new(0, 3, "xxx")]),
        make_violation(file.clone(), vec![Fix::new(8, 11, "zzz")]),
    ];

    let count = fixer::apply_fixes(&violations).unwrap();
    assert_eq!(count, 2);
    assert_eq!(fs::read_to_string(&file).unwrap(), "xxx bbb zzz");
}

#[test]
fn overlapping_fixes_from_different_violations_skip_the_later_one() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.c");
    fs::write(&file, "hello world test").unwrap();

    // Two violations with overlapping byte ranges
    let violations = vec![
        make_violation(file.clone(), vec![Fix::new(0, 11, "goodbye")]),
        make_violation(file.clone(), vec![Fix::new(6, 16, "planet")]),
    ];

    let count = fixer::apply_fixes(&violations).unwrap();
    // Both violations count as having fixes
    assert_eq!(count, 2);
    // The higher start_byte fix (6..16) is applied first (sorted descending),
    // then the lower one (0..11) overlaps and is skipped.
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content, "hello planet");
}

#[test]
fn multiple_fixes_in_same_violation_not_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("test.c");
    fs::write(&file, "aaa bbb ccc").unwrap();

    // Single violation with two non-overlapping fixes (same group_index)
    let violations = vec![make_violation(
        file.clone(),
        vec![Fix::new(0, 3, "xxx"), Fix::new(8, 11, "zzz")],
    )];

    let count = fixer::apply_fixes(&violations).unwrap();
    assert_eq!(count, 1);
    assert_eq!(fs::read_to_string(&file).unwrap(), "xxx bbb zzz");
}

#[test]
fn no_fixes_returns_zero() {
    let violations = vec![make_violation(PathBuf::from("/nonexistent"), vec![])];

    let count = fixer::apply_fixes(&violations).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn empty_violations_returns_zero() {
    let count = fixer::apply_fixes(&[]).unwrap();
    assert_eq!(count, 0);
}
