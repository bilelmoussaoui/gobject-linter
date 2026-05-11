use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::rules::{Fix, Violation};

/// Apply fixes to files
pub fn apply_fixes(violations: &[Violation]) -> Result<usize> {
    // Count violations with fixes (not individual fixes)
    let total_violations_with_fixes = violations.iter().filter(|v| !v.fixes.is_empty()).count();

    // Collect all fixes from all violations, grouped by file
    let mut by_file: HashMap<&Path, Vec<&Fix>> = HashMap::new();
    for violation in violations {
        if !violation.fixes.is_empty() {
            for fix in &violation.fixes {
                by_file
                    .entry(violation.file.as_path())
                    .or_default()
                    .push(fix);
            }
        }
    }

    for (file_path, mut fixes) in by_file {
        // Sort by start_byte descending - apply fixes from bottom to top
        // This way earlier fixes don't invalidate byte positions of later fixes
        fixes.sort_by_key(|b| std::cmp::Reverse(b.start_byte));

        // Read file content as bytes
        let content = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let mut modified_content = content;

        // Track the lowest byte offset touched so far (we apply top-down in
        // the reversed list, i.e. from high offsets to low). Any fix whose
        // range overlaps into the already-modified region is skipped.
        let mut protected_boundary: Option<usize> = None;

        // Apply each fix
        for fix in fixes {
            if let Some(boundary) = protected_boundary
                && fix.end_byte > boundary
            {
                eprintln!(
                    "{}: skipping overlapping fix in {} (bytes {}..{} overlaps with already-applied fix at byte {}); re-run --fix to apply",
                    "warning".yellow(),
                    file_path.display(),
                    fix.start_byte,
                    fix.end_byte,
                    boundary,
                );
                continue;
            }

            protected_boundary = Some(fix.start_byte);

            // Replace the range [start_byte, end_byte) with replacement
            let mut new_content = Vec::new();
            new_content.extend_from_slice(&modified_content[..fix.start_byte]);
            new_content.extend_from_slice(fix.replacement.as_bytes());
            new_content.extend_from_slice(&modified_content[fix.end_byte..]);

            modified_content = new_content;
        }

        // Write back to file
        fs::write(file_path, modified_content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
    }

    Ok(total_violations_with_fixes)
}
