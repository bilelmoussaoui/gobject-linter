use std::path::PathBuf;

use gobject_ast::{CommentKind, CommentPosition, Parser};

fn parse_fixture(name: &str) -> gobject_ast::Project {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    let mut parser = Parser::new().unwrap();
    parser.parse_file(&fixture_path).unwrap()
}

#[test]
fn test_parse_line_comments() {
    let project = parse_fixture("comments.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("comments.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    // Should have extracted comments
    assert!(
        !file.comments.is_empty(),
        "Should have extracted comments from file"
    );

    // Find the line comment
    let line_comment = file
        .comments
        .iter()
        .find(|c| matches!(c.kind, CommentKind::Line))
        .expect("Should find line comment");

    assert!(
        line_comment.text.contains("This is a line comment"),
        "Comment text should be extracted without // delimiter"
    );
}

#[test]
fn test_parse_block_comments() {
    let project = parse_fixture("comments.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("comments.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    // Find the block comment
    let block_comment = file
        .comments
        .iter()
        .find(|c| matches!(c.kind, CommentKind::Block))
        .expect("Should find block comment");

    assert!(
        block_comment.text.contains("This is a block comment"),
        "Comment text should be extracted without /* */ delimiters"
    );
}

#[test]
fn test_comment_helpers() {
    let project = parse_fixture("comments.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("comments.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    // Test contains helper
    let todo_comment = file
        .comments
        .iter()
        .find(|c| c.contains("TODO"))
        .expect("Should find TODO comment");

    assert!(
        todo_comment.is_marker(),
        "TODO should be recognized as marker"
    );

    // Test extract_ignore_rules
    let ignore_comment = file
        .comments
        .iter()
        .find(|c| c.contains("gobject-linter-ignore"))
        .expect("Should find goblint-ignore comment");

    let rules = ignore_comment
        .extract_ignore_rules()
        .expect("Should extract rule names");

    assert!(
        rules.contains(&"rule_name".to_string()),
        "Should extract rule names from ignore directive"
    );
}

#[test]
fn test_comment_positions() {
    let project = parse_fixture("comment_positions.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("comment_positions.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    // Should have extracted all comments
    assert!(file.comments.len() >= 3, "Should have at least 3 comments");

    // Find trailing comment (same line as code)
    let trailing = file
        .comments
        .iter()
        .find(|c| c.text.contains("Trailing comment on same line"))
        .expect("Should find trailing comment");

    assert!(
        matches!(trailing.position, CommentPosition::Trailing),
        "Comment on same line as code should be Trailing, got {:?}",
        trailing.position
    );

    // Find leading comments (before code)
    let leading = file
        .comments
        .iter()
        .find(|c| c.text.contains("Leading comment before statement"))
        .expect("Should find leading comment");

    assert!(
        matches!(leading.position, CommentPosition::Leading),
        "Comment on line before code should be Leading, got {:?}",
        leading.position
    );
}
