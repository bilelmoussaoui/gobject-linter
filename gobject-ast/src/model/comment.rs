use serde::Serialize;

use crate::SourceLocation;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommentKind {
    /// Single-line comment: // ...
    Line,
    /// Multi-line comment: /* ... */
    Block,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommentPosition {
    /// Comment appears before the associated node
    Leading,
    /// Comment appears after the associated node (same line)
    Trailing,
    /// Comment appears inside a node (e.g., between statements)
    Inner,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    /// The comment text (without // or /* */ delimiters)
    pub text: String,
    /// Location in source
    pub location: SourceLocation,
    /// Line or block comment
    pub kind: CommentKind,
    /// Position relative to associated node
    pub position: CommentPosition,
}

impl Comment {
    pub fn new(
        text: String,
        location: SourceLocation,
        kind: CommentKind,
        position: CommentPosition,
    ) -> Self {
        Self {
            text,
            location,
            kind,
            position,
        }
    }

    /// Check if comment contains a specific annotation (case-insensitive)
    pub fn contains(&self, pattern: &str) -> bool {
        self.text.to_lowercase().contains(&pattern.to_lowercase())
    }

    /// Extract gobject-linter-ignore rule names from comment
    /// Returns Some(vec![rule_names]) if this is an ignore directive
    pub fn extract_ignore_rules(&self) -> Option<Vec<String>> {
        let text = self.text.trim();

        // Match: gobject-linter-ignore: rule1, rule2
        // Or: gobject-linter-ignore-next-line: rule1
        if let Some(after_prefix) = text
            .strip_prefix("gobject-linter-ignore:")
            .or_else(|| text.strip_prefix("gobject-linter-ignore-next-line:"))
            .or_else(|| text.strip_prefix("goblint-ignore:"))
            .or_else(|| text.strip_prefix("goblint-ignore-next-line:"))
        {
            let rules: Vec<String> = after_prefix
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return Some(rules);
        }

        None
    }

    /// Check if this is a GTK-Doc style documentation comment
    pub fn is_gtk_doc(&self) -> bool {
        matches!(self.kind, CommentKind::Block) && self.text.trim_start().starts_with('*')
    }

    /// Check if this is a TODO/FIXME/HACK/XXX comment
    pub fn is_marker(&self) -> bool {
        let upper = self.text.to_uppercase();
        upper.contains("TODO")
            || upper.contains("FIXME")
            || upper.contains("HACK")
            || upper.contains("XXX")
    }
}
