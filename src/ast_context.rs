use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::Result;
use globset::GlobSet;
use gobject_ast::{FileModel, Project, parser::Parser};
use ignore::WalkBuilder;
use indicatif::ProgressBar;
use rayon::prelude::*;

use crate::{meson::MesonHeaders, type_alias_map::TypeAliasMap};

/// AST-based project context that replaces the old tree-sitter based
/// ProjectContext
pub struct AstContext {
    pub project: Project,
    /// Header visibility from meson introspection.
    /// None means no meson info was available
    pub meson_headers: Option<MesonHeaders>,
    type_aliases: TypeAliasMap,
}

impl AstContext {
    /// Build with ignore patterns and optional meson header visibility info
    pub fn build_with_ignore(
        directory: &Path,
        ignore_matcher: &GlobSet,
        spinner: Option<&ProgressBar>,
        meson_headers: Option<MesonHeaders>,
    ) -> Result<Self> {
        // Collect all files first to get count
        // WalkBuilder respects .gitignore, .ignore, and other ignore files
        // automatically
        let files: Vec<_> = WalkBuilder::new(directory)
            .hidden(false) // Include hidden files/dirs
            .git_ignore(true) // Respect .gitignore
            .git_global(true) // Respect global gitignore
            .git_exclude(true) // Respect .git/info/exclude
            .require_git(false) // Work in non-git directories too
            .build()
            .filter_map(std::result::Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "h" || ext == "c")
            })
            .filter(|e| {
                let path = e.path();
                let relative_path = path.strip_prefix(directory).unwrap_or(path);
                !ignore_matcher.is_match(relative_path)
            })
            .collect();

        let total_files = files.len();
        let counter = AtomicUsize::new(0);

        // Parse files in parallel
        let chunks: Vec<Vec<_>> = files
            .par_iter()
            .fold(
                || (Parser::new().unwrap(), Vec::new()),
                |(mut parser, mut results), entry| {
                    let i = counter.fetch_add(1, Ordering::Relaxed);
                    if let Some(sp) = spinner {
                        sp.set_message(format!("Parsing files... {}/{}", i + 1, total_files));
                    }
                    if let Ok(item) = parser.parse_file_to_model(entry.path()) {
                        results.push(item);
                    }
                    (parser, results)
                },
            )
            .map(|(_, results)| results)
            .collect();

        let mut project = Project::new();
        project.files.reserve(total_files);
        for chunk in chunks {
            for (path, model) in chunk {
                project.files.insert(path, model);
            }
        }

        let type_aliases = TypeAliasMap::build(&project);

        Ok(Self {
            project,
            meson_headers,
            type_aliases,
        })
    }

    /// Update a single file in the project
    pub fn update_file(&mut self, file_path: &Path) -> Result<()> {
        let mut parser = Parser::new()?;

        // Parse the file
        if let Ok(file_project) = parser.parse_file(file_path) {
            // Update or insert the file in the project
            for (path, file_model) in file_project.files {
                self.project.files.insert(path, file_model);
            }
        } else {
            // If parsing failed, remove the file from the project
            self.project.files.remove(file_path);
        }

        self.type_aliases = TypeAliasMap::build(&self.project);

        Ok(())
    }

    /// Typedef/struct-tag alias map for the whole project.
    pub fn type_aliases(&self) -> &TypeAliasMap {
        &self.type_aliases
    }

    /// Iterate over all files in the project
    pub fn iter_all_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project
            .files
            .iter()
            .map(|(path, file)| (path.as_path(), file))
    }

    /// Iterate over all C files (extension .c) in the project
    pub fn iter_c_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project
            .files
            .iter()
            .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "c"))
            .map(|(path, file)| (path.as_path(), file))
    }

    /// Iterate over all header files (extension .h) in the project
    pub fn iter_header_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project
            .files
            .iter()
            .filter(|(path, _)| path.extension().is_some_and(|ext| ext == "h"))
            .map(|(path, file)| (path.as_path(), file))
    }

    /// Iterate over all files that are not part of the public API: all `.c`
    /// files and any `.h` files that meson does not mark as installed/public.
    pub fn iter_private_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project.files.iter().filter_map(|(path, file)| {
            let p = path.as_path();
            if path.extension().is_some_and(|ext| ext == "c") {
                return Some((p, file));
            }
            if self.is_public_header(p) == Some(true) {
                return None;
            }
            Some((p, file))
        })
    }

    /// Check if a file path is a public header.
    /// A header is public if it appears in either the GIR filelist or the
    /// installed headers set
    /// Returns None if no meson info is available at all.
    pub fn is_public_header(&self, path: &Path) -> Option<bool> {
        let h = self.meson_headers.as_ref()?;
        Some(h.gir.contains(path) || h.installed.contains(path))
    }

    /// Check if a file path is a header passed to g-ir-scanner.
    pub fn is_gir_header(&self, path: &Path) -> Option<bool> {
        let h = self.meson_headers.as_ref()?;
        Some(h.gir.contains(path))
    }

    /// Check if public/private distinction is available
    pub fn has_public_private_info(&self) -> bool {
        self.meson_headers.is_some()
    }
}
