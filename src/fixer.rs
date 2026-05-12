use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::rules::{Fix, Violation};

/// Apply fixes to files
pub fn apply_fixes(violations: &[Violation]) -> Result<usize> {
    // Count violations with fixes (not individual fixes)
    let total_violations_with_fixes = violations.iter().filter(|v| !v.fixes.is_empty()).count();

    // Collect fix groups per file
    let mut by_file: HashMap<&Path, Vec<&[Fix]>> = HashMap::new();
    for violation in violations {
        if !violation.fixes.is_empty() {
            by_file
                .entry(violation.file.as_path())
                .or_default()
                .push(&violation.fixes);
        }
    }

    for (file_path, fix_groups) in by_file {
        // Flatten into (fix, group_index) pairs sorted by start_byte descending
        let mut tagged_fixes: Vec<(&Fix, usize)> = fix_groups
            .iter()
            .enumerate()
            .flat_map(|(group_idx, fixes)| fixes.iter().map(move |fix| (fix, group_idx)))
            .collect();
        tagged_fixes.sort_by_key(|(fix, _)| std::cmp::Reverse(fix.start_byte));

        // Read file content as bytes
        let content = fs::read(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let mut modified_content = content;

        // Track the lowest byte offset touched so far (we apply from high
        // offsets to low). When a fix from a *different* group overlaps into
        // the already-modified region it is skipped.
        let mut protected_boundary: Option<(usize, usize)> = None; // (byte, group_index)

        for (fix, group_idx) in tagged_fixes {
            if let Some((boundary, boundary_group)) = protected_boundary
                && group_idx != boundary_group
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

            protected_boundary = Some((fix.start_byte, group_idx));

            // Replace the range [start_byte, end_byte) with replacement
            let mut new_content = Vec::new();
            new_content.extend_from_slice(&modified_content[..fix.start_byte]);
            if let Some(ref replacement) = fix.replacement {
                new_content.extend_from_slice(replacement.as_bytes());
            }
            new_content.extend_from_slice(&modified_content[fix.end_byte..]);

            modified_content = new_content;
        }

        // Write back to file
        fs::write(file_path, modified_content)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;
    }

    Ok(total_violations_with_fixes)
}
