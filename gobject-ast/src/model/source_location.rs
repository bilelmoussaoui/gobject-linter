use std::fmt;

use serde::{Serialize, Serializer};

/// Source location information for AST nodes
#[derive(Clone, Copy, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
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
    pub fn new(line: usize, column: usize, start_byte: usize, end_byte: usize) -> Self {
        Self {
            line,
            column,
            start_byte,
            end_byte,
        }
    }

    /// Extract the source text for this location
    pub fn as_str<'a>(&self, source: &'a [u8]) -> Option<&'a str> {
        std::str::from_utf8(&source[self.start_byte..self.end_byte]).ok()
    }

    /// Find the byte position of the start of the line containing this location
    fn find_line_start(&self, source: &[u8]) -> usize {
        let mut line_start = self.start_byte;
        while line_start > 0 && source[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        line_start
    }

    /// Find the start and end byte positions of the line containing this
    /// location Returns (line_start_byte, line_end_byte) including the
    /// newline If the previous line is empty (only whitespace), includes it
    /// too
    pub fn find_line_bounds(&self, source: &[u8]) -> (usize, usize) {
        // Find the start of the line
        let mut line_start = self.find_line_start(source);

        // Check if the previous line is empty (only whitespace)
        if line_start > 0 {
            let mut prev_line_start = line_start - 1; // Skip the '\n'
            while prev_line_start > 0 && source[prev_line_start - 1] != b'\n' {
                prev_line_start -= 1;
            }

            // Check if the line is only whitespace
            let prev_line = &source[prev_line_start..line_start - 1];
            if prev_line.iter().all(|&b| b == b' ' || b == b'\t') {
                line_start = prev_line_start;
            }
        }

        // Find the end of the line (including newline)
        let mut line_end = self.start_byte;
        while line_end < source.len() && source[line_end] != b'\n' {
            line_end += 1;
        }
        // Include the newline character
        if line_end < source.len() && source[line_end] == b'\n' {
            line_end += 1;
        }

        (line_start, line_end)
    }

    /// Find the start and end byte positions of the line containing this
    /// location, including any following blank line
    /// Returns (line_start_byte, line_end_byte) including newlines
    pub fn find_line_bounds_with_following_blank(&self, source: &[u8]) -> (usize, usize) {
        // Find the start of the line
        let line_start = self.find_line_start(source);

        // Find the end of the line (including newline)
        let mut line_end = self.end_byte;
        while line_end < source.len() && source[line_end] != b'\n' {
            line_end += 1;
        }
        if line_end < source.len() {
            line_end += 1; // Include the newline
        }

        // Check if the next line is blank (only whitespace)
        if line_end < source.len() {
            let mut pos = line_end;
            let mut is_blank = true;

            while pos < source.len() && source[pos] != b'\n' {
                if !source[pos].is_ascii_whitespace() {
                    is_blank = false;
                    break;
                }
                pos += 1;
            }

            // If next line is blank, include it in the removal
            if is_blank && pos < source.len() {
                line_end = pos + 1; // Include the newline of the blank line
            }
        }

        (line_start, line_end)
    }

    /// Extract indentation (leading whitespace) from the line containing this
    /// location. Returns all leading spaces/tabs from the start of the line.
    pub fn extract_line_indentation(&self, source: &[u8]) -> String {
        let line_start = self.find_line_start(source);

        // Extract all leading whitespace from the line
        let mut indent = String::new();
        let mut i = line_start;
        while i < source.len() && (source[i] == b' ' || source[i] == b'\t') {
            indent.push(source[i] as char);
            i += 1;
        }

        indent
    }

    /// Extract indentation (leading whitespace) up to this location
    /// Returns the spaces/tabs from line start up to (but not past) the
    /// location
    pub fn extract_indentation(&self, source: &[u8]) -> String {
        let line_start = self.find_line_start(source);

        // Extract indentation (spaces/tabs before first non-whitespace or before
        // location)
        let mut indent = String::new();
        for &byte in &source[line_start..self.start_byte] {
            if byte == b' ' || byte == b'\t' {
                indent.push(byte as char);
            } else {
                break;
            }
        }

        indent
    }

    /// Scan forward from `end_byte` through whitespace to find a `;` and
    /// return the byte position immediately after it. Returns `end_byte`
    /// unchanged if no semicolon is found within a short distance.
    pub fn find_semicolon_end(&self, source: &[u8]) -> usize {
        let mut pos = self.end_byte;
        while pos < source.len() {
            match source[pos] {
                b';' => return pos + 1,
                b' ' | b'\t' | b'\r' | b'\n' => pos += 1,
                _ => break,
            }
        }
        self.end_byte
    }

    /// Find braces surrounding a range in the source
    /// Returns (opening_brace_pos, closing_brace_pos) using depth tracking to
    /// find matching braces
    pub fn find_braces_around(start: usize, source: &[u8]) -> (usize, usize) {
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
