use std::{fmt, sync::Arc};

use serde::{Serialize, Serializer};

/// Source location information for AST nodes
#[derive(Clone, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    source: Arc<Vec<u8>>,
}

impl fmt::Debug for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Serialize for SourceLocation {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{}:{}", self.line, self.column))
    }
}

impl SourceLocation {
    pub fn new(
        line: usize,
        column: usize,
        start_byte: usize,
        end_byte: usize,
        source: Arc<Vec<u8>>,
    ) -> Self {
        Self {
            line,
            column,
            start_byte,
            end_byte,
            source,
        }
    }

    /// Create a new `SourceLocation` sharing the same source but pointing at
    /// a different byte range. Line/column are zeroed since they are only used
    /// for display and these synthetic locations are used for fix generation.
    pub fn with_byte_range(&self, start_byte: usize, end_byte: usize) -> Self {
        Self {
            line: 0,
            column: 0,
            start_byte,
            end_byte,
            source: Arc::clone(&self.source),
        }
    }

    /// Get the shared source bytes
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    /// Extract the source text for this location
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.source[self.start_byte..self.end_byte]).ok()
    }

    /// Find the byte position of the start of the line containing this location
    fn find_line_start(&self) -> usize {
        let mut line_start = self.start_byte;
        while line_start > 0 && self.source[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        line_start
    }

    /// Find the byte position just past the newline at the end of the
    /// statement/expression. Scans forward from `end_byte`.
    fn find_line_end(&self) -> usize {
        let mut pos = self.end_byte;
        while pos < self.source.len() && self.source[pos] != b'\n' {
            pos += 1;
        }
        if pos < self.source.len() {
            pos += 1;
        }
        pos
    }

    /// Return (start, end) covering the full line from its first byte
    /// to just past the trailing newline — no blank-line absorption.
    pub fn find_line_range(&self) -> (usize, usize) {
        (self.find_line_start(), self.find_line_end())
    }

    /// Find the start and end byte positions of the line containing this
    /// location Returns (line_start_byte, line_end_byte) including the
    /// newline If the previous line is empty (only whitespace), includes it
    /// too
    pub fn find_line_bounds(&self) -> (usize, usize) {
        // Find the start of the line
        let mut line_start = self.find_line_start();

        // Check if the previous line is empty (only whitespace)
        if line_start > 0 {
            let mut prev_line_start = line_start - 1; // Skip the '\n'
            while prev_line_start > 0 && self.source[prev_line_start - 1] != b'\n' {
                prev_line_start -= 1;
            }

            // Check if the line is only whitespace
            let prev_line = &self.source[prev_line_start..line_start - 1];
            if prev_line.iter().all(|&b| b == b' ' || b == b'\t') {
                line_start = prev_line_start;
            }
        }

        // Find the end of the line (including newline)
        let mut line_end = self.start_byte;
        while line_end < self.source.len() && self.source[line_end] != b'\n' {
            line_end += 1;
        }
        // Include the newline character
        if line_end < self.source.len() && self.source[line_end] == b'\n' {
            line_end += 1;
        }

        (line_start, line_end)
    }

    /// Find the start and end byte positions of the line containing this
    /// location, including any following blank line
    /// Returns (line_start_byte, line_end_byte) including newlines
    pub fn find_line_bounds_with_following_blank(&self) -> (usize, usize) {
        let line_start = self.find_line_start();
        let mut line_end = self.find_line_end();

        // Check if the next line is blank (only whitespace)
        if line_end < self.source.len() {
            let mut pos = line_end;
            let mut is_blank = true;

            while pos < self.source.len() && self.source[pos] != b'\n' {
                if !self.source[pos].is_ascii_whitespace() {
                    is_blank = false;
                    break;
                }
                pos += 1;
            }

            // If next line is blank, include it in the removal
            if is_blank && pos < self.source.len() {
                line_end = pos + 1; // Include the newline of the blank line
            }
        }

        (line_start, line_end)
    }

    /// Extract indentation (leading whitespace) from the line containing this
    /// location. Returns all leading spaces/tabs from the start of the line.
    pub fn extract_line_indentation(&self) -> String {
        let line_start = self.find_line_start();

        // Extract all leading whitespace from the line
        let mut indent = String::new();
        let mut i = line_start;
        while i < self.source.len() && (self.source[i] == b' ' || self.source[i] == b'\t') {
            indent.push(self.source[i] as char);
            i += 1;
        }

        indent
    }

    /// Extract indentation (leading whitespace) up to this location
    /// Returns the spaces/tabs from line start up to (but not past) the
    /// location
    pub fn extract_indentation(&self) -> String {
        let line_start = self.find_line_start();

        // Extract indentation (spaces/tabs before first non-whitespace or before
        // location)
        let mut indent = String::new();
        for &byte in &self.source[line_start..self.start_byte] {
            if byte == b' ' || byte == b'\t' {
                indent.push(byte as char);
            } else {
                break;
            }
        }

        indent
    }

    /// Scan forward from `end_byte` through whitespace to find `target` and
    /// return the byte position immediately after it. Returns `end_byte`
    /// unchanged if the target is not found before a non-whitespace byte.
    pub fn find_after(&self, target: u8) -> usize {
        let mut pos = self.end_byte;
        while pos < self.source.len() {
            if self.source[pos] == target {
                return pos + 1;
            }
            if !self.source[pos].is_ascii_whitespace() {
                break;
            }
            pos += 1;
        }
        self.end_byte
    }

    /// Scan backward from `start_byte` through whitespace to find `target` and
    /// return the byte position of the target. Returns `start_byte`
    /// unchanged if the target is not found before a non-whitespace byte.
    pub fn find_before(&self, target: u8) -> usize {
        let mut pos = self.start_byte;
        while pos > 0 && self.source[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        if pos > 0 && self.source[pos - 1] == target {
            pos -= 1;
            // Also skip whitespace before the target
            while pos > 0 && self.source[pos - 1] == b' ' {
                pos -= 1;
            }
        }
        pos
    }

    /// Convenience: scan forward for `;` — equivalent to `find_after(b';')`
    pub fn find_semicolon_end(&self) -> usize {
        self.find_after(b';')
    }

    /// Count consecutive newlines immediately after `end_byte`.
    pub fn count_trailing_newlines(&self) -> usize {
        self.source[self.end_byte..]
            .iter()
            .take_while(|&&b| b == b'\n')
            .count()
    }

    /// Find braces surrounding this location in the source.
    /// Returns (opening_brace_pos, closing_brace_pos) using depth tracking to
    /// find matching braces.
    pub fn find_braces_around(&self) -> (usize, usize) {
        let source = &self.source;
        let start = self.start_byte;

        // Search backwards from start to find '{'
        let mut brace_start = start;
        while brace_start > 0 && source[brace_start - 1] != b'{' {
            brace_start -= 1;
            if start - brace_start > 100 {
                break;
            }
        }
        if brace_start > 0 && source[brace_start - 1] == b'{' {
            brace_start -= 1;
        }

        // Search forwards from opening brace to find matching closing brace using depth
        // tracking
        let mut brace_end = brace_start + 1;
        let mut depth = 1;
        while brace_end < source.len() && depth > 0 {
            if source[brace_end] == b'{' {
                depth += 1;
            } else if source[brace_end] == b'}' {
                depth -= 1;
            }
            brace_end += 1;
        }

        (brace_start, brace_end)
    }
}
