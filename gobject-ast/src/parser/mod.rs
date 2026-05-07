mod expression;
mod gobject;
mod statement;
mod top_level;

use std::{fs, path::Path};

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use tree_sitter::{Node, Parser as TSParser};

use crate::model::*;

pub struct Parser {
    parser: TSParser,
    current_file: Option<std::path::PathBuf>,
}

impl Parser {
    pub fn new() -> Result<Self> {
        let mut parser = TSParser::new();
        parser
            .set_language(&tree_sitter_c_gobject::LANGUAGE.into())
            .context("Failed to load C grammar")?;

        Ok(Self {
            parser,
            current_file: None,
        })
    }

    /// Helper to create SourceLocation from a tree-sitter Node
    fn node_location(&self, node: Node) -> SourceLocation {
        SourceLocation::new(
            node.start_position().row + 1,
            node.start_position().column + 1,
            node.start_byte(),
            node.end_byte(),
        )
    }

    /// Check if a tree-sitter node is an expression
    fn is_expression_node(node: &Node) -> bool {
        matches!(
            node.kind(),
            "call_expression"
                | "assignment_expression"
                | "binary_expression"
                | "unary_expression"
                | "pointer_expression"
                | "parenthesized_expression"
                | "identifier"
                | "field_expression"
                | "string_literal"
                | "number_literal"
                | "null"
                | "NULL"
                | "true"
                | "TRUE"
                | "false"
                | "FALSE"
                | "cast_expression"
                | "conditional_expression"
                | "sizeof_expression"
                | "alignof_expression"
                | "subscript_expression"
                | "initializer_list"
                | "char_literal"
                | "update_expression"
                | "concatenated_string"
                | "compound_literal_expression"
                | "comma_expression"
                | "offsetof_expression"
                | "gnu_asm_expression"
                | "compound_statement"
                | "comment"
        )
    }

    pub fn parse_directory(&mut self, path: &Path) -> Result<Project> {
        let mut project = Project::new();

        // Parse all files (.h and .c)
        // WalkBuilder respects .gitignore by default
        for entry in WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .require_git(false)
            .build()
            .filter_map(std::result::Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "h" || ext == "c")
            })
        {
            self.parse_single_file(entry.path(), &mut project)?;
        }

        Ok(project)
    }

    pub fn parse_file(&mut self, path: &Path) -> Result<Project> {
        let mut project = Project::new();
        self.parse_single_file(path, &mut project)?;
        Ok(project)
    }

    fn parse_single_file(&mut self, path: &Path, project: &mut Project) -> Result<()> {
        let _file_span = tracing::warn_span!("file", path = %path.display()).entered();
        self.current_file = Some(path.to_path_buf());
        let source = fs::read(path)?;
        let tree = self
            .parser
            .parse(&source, None)
            .context("Failed to parse file")?;

        let mut file_model = FileModel::new(path.to_path_buf());

        // Extract all content from this file
        self.visit_node(tree.root_node(), &source, &mut file_model);

        // Store the source for detailed pattern matching by rules
        file_model.source = source;

        file_model.resolve_gobject_types();

        project.files.insert(path.to_path_buf(), file_model);
        Ok(())
    }

    fn find_export_macros_in_declaration<'a>(
        &self,
        decl_node: Node,
        source: &'a [u8],
    ) -> Vec<&'a str> {
        let mut result = Vec::new();

        // With the fixed grammar, macro_modifier nodes are now properly parsed
        // Just walk children and extract macro_modifier nodes
        let mut cursor = decl_node.walk();

        for child in decl_node.children(&mut cursor) {
            if child.kind() == "macro_modifier" {
                let text = std::str::from_utf8(&source[child.byte_range()]).unwrap_or("");
                result.push(text.trim());
            }
        }

        result
    }

    /// Extract GObject type from a gobject_type_macro or macro_modifier node
    /// The grammar now properly parses argument_list with identifier children
    fn extract_gobject_from_macro_modifier(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<GObjectType> {
        // Collect export macros from gobject_export_macro children
        let export_macros: Vec<String> = {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter(|c| c.kind() == "gobject_export_macro")
                .filter_map(|c| std::str::from_utf8(&source[c.byte_range()]).ok())
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        };

        // Get macro name from the text (before parentheses)
        let full_text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
        let macro_name = full_text.split('(').next()?.trim();
        // Strip any leading export macro prefix to get the actual macro name
        let macro_name = macro_name.split_whitespace().last()?;

        let mut gobject_type =
            self.extract_gobject_from_identifier(node, node, source, macro_name)?;
        gobject_type.export_macros = export_macros;
        Some(gobject_type)
    }

    fn visit_node(&self, node: Node, source: &[u8], file_model: &mut FileModel) {
        // Try to parse this node as a top-level item. If successful, don't
        // recurse — children are handled inside parse_top_level_item itself
        // (e.g. via parse_conditional_body for #ifdef blocks).
        if let Some(item) = self.parse_top_level_item(node, source) {
            file_model.top_level_items.push(item);
            return;
        }

        // Only recurse when the node wasn't recognized as a top-level item
        // (translation_unit, unrecognized wrapper nodes, etc.)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child, source, file_model);
        }
    }

    fn extract_function_from_definition<'a>(
        &self,
        node: Node,
        source: &'a [u8],
    ) -> Option<(&'a str, bool, bool)> {
        let func_text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
        let is_static = func_text.starts_with("static") || func_text.contains("\nstatic ");
        let is_inline = func_text.contains("inline ");

        let declarator = node.child_by_field_name("declarator")?;
        let name = self.extract_declarator_name(declarator, source)?;

        Some((name, is_static, is_inline))
    }

    pub(super) fn find_function_declarator<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        if node.kind() == "function_declarator" {
            return Some(node);
        }

        // For pointer/abstract declarators, look in the declarator field
        if let Some(declarator) = node.child_by_field_name("declarator")
            && let Some(found) = self.find_function_declarator(declarator)
        {
            return Some(found);
        }

        // Recursively search children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_function_declarator(child) {
                return Some(found);
            }
        }

        None
    }

    pub(super) fn extract_comment_text(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<(CommentKind, String)> {
        let text = std::str::from_utf8(&source[node.byte_range()]).ok()?;

        if text.starts_with("//") {
            Some((CommentKind::Line, text.to_string()))
        } else if text.starts_with("/*") && text.ends_with("*/") {
            Some((CommentKind::Block, text.to_string()))
        } else {
            None
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new().expect("Failed to create parser")
    }
}
