use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::Result;
use globset::GlobSet;
use gobject_ast::{
    Parser,
    model::{FileModel, Project},
};
use ignore::WalkBuilder;
use indicatif::ProgressBar;
use rayon::prelude::*;

use crate::{meson::MesonIntrospection, type_alias_map::TypeAliasMap};

/// AST-based project context that replaces the old tree-sitter based
/// ProjectContext
pub struct AstContext {
    pub project: Project,
    /// Meson introspection data.
    /// None means no meson info was available
    pub meson_introspection: Option<MesonIntrospection>,
    type_aliases: TypeAliasMap,
}

impl AstContext {
    /// Build with ignore patterns and optional meson introspection
    pub fn build_with_ignore(
        directory: &Path,
        ignore_matcher: &GlobSet,
        spinner: Option<&ProgressBar>,
        meson_introspection: Option<MesonIntrospection>,
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

        project.resolve_all_gobject_types();
        let type_aliases = TypeAliasMap::build(&project);

        Ok(Self {
            project,
            meson_introspection,
            type_aliases,
        })
    }

    /// Update a single file in the project
    pub fn update_file(&mut self, file_path: &Path) -> Result<()> {
        let mut parser = Parser::new()?;

        if let Ok((path, model)) = parser.parse_file_to_model(file_path) {
            self.project.files.insert(path, model);
        } else {
            self.project.files.remove(file_path);
        }

        self.project.resolve_all_gobject_types();
        self.type_aliases = TypeAliasMap::build(&self.project);

        Ok(())
    }

    /// Typedef/struct-tag alias map for the whole project.
    pub fn type_aliases(&self) -> &TypeAliasMap {
        &self.type_aliases
    }

    pub fn iter_all_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project.iter_all_files()
    }

    pub fn iter_c_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project.iter_c_files()
    }

    pub fn iter_header_files(&self) -> impl Iterator<Item = (&Path, &FileModel)> {
        self.project.iter_header_files()
    }

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

    pub fn is_public_header(&self, path: &Path) -> Option<bool> {
        let m = self.meson_introspection.as_ref()?;
        let gir = m.get_introspected_headers();
        let installed = m.get_installed_headers();
        Some(gir.contains(path) || installed.contains(path))
    }

    pub fn is_gir_header(&self, path: &Path) -> Option<bool> {
        let m = self.meson_introspection.as_ref()?;
        let gir = m.get_introspected_headers();
        Some(gir.contains(path))
    }

    pub fn has_public_private_info(&self) -> bool {
        self.meson_introspection.is_some()
    }

    pub fn find_func_doc(&self, name: &str) -> Option<&gobject_ast::model::FunctionDoc> {
        self.project.find_func_doc(name)
    }

    pub fn find_type_doc(&self, type_name: &str) -> Option<&gobject_ast::model::TypeDoc> {
        self.project.find_type_doc(type_name)
    }

    pub fn find_gobject_type_by_gtype(
        &self,
        gtype: &gobject_ast::model::GType,
    ) -> Option<&gobject_ast::model::GObjectType> {
        self.project.find_gobject_type_by_gtype(gtype)
    }
}
