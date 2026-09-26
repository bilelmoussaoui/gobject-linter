use tree_sitter::Node;

use crate::{
    model::{AutoCleanupMacro, CompoundStatement, Statement, TypeInfo},
    parser::Parser,
};

enum TagKeyword {
    Struct,
    Union,
    Enum,
}

impl Parser {
    pub(crate) fn parse_compound_statement(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<CompoundStatement> {
        let statements = self.parse_function_body(node, source);

        Some(CompoundStatement {
            statements,
            location: self.node_location(node),
        })
    }

    pub(crate) fn parse_function_body(&self, body_node: Node, source: &[u8]) -> Vec<Statement> {
        let mut statements = Vec::new();
        // tree-sitter splits `g_autofree struct Foo *var = NULL;` into two
        // declaration nodes: (1) `g_autofree struct` and (2) `Foo *var = NULL`.
        // The first fails to parse (no variable name), so we detect the
        // pattern and carry the struct/union/enum + auto_cleanup to the next
        // one.
        let mut pending_struct_fixup: Option<(TagKeyword, AutoCleanupMacro)> = None;

        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            if child.kind() == "declaration" {
                let decls = self.parse_variable_decls(child, source);
                if decls.is_empty() {
                    pending_struct_fixup = Self::detect_autofree_struct_stub(child, source);
                } else {
                    for (i, mut decl) in decls.into_iter().enumerate() {
                        if i == 0
                            && let Some((tag, auto_cleanup)) = pending_struct_fixup.take()
                        {
                            match tag {
                                TagKeyword::Struct => decl.type_info.is_struct = true,
                                TagKeyword::Union => decl.type_info.is_union = true,
                                TagKeyword::Enum => decl.type_info.is_enum = true,
                            }
                            decl.type_info.auto_cleanup = Some(auto_cleanup);
                        }
                        statements.push(Statement::Declaration(Box::new(decl)));
                    }
                }
            } else if let Some(stmt) = self.parse_statement(child, source) {
                pending_struct_fixup = None;
                statements.push(stmt);
            }
        }

        statements
    }

    fn detect_autofree_struct_stub(
        node: Node,
        source: &[u8],
    ) -> Option<(TagKeyword, AutoCleanupMacro)> {
        let text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
        let auto_cleanup = TypeInfo::parse_auto_cleanup(text)?;
        let trimmed = text.trim().trim_end_matches(';').trim();
        if trimmed.ends_with(" struct") {
            Some((TagKeyword::Struct, auto_cleanup))
        } else if trimmed.ends_with(" union") {
            Some((TagKeyword::Union, auto_cleanup))
        } else if trimmed.ends_with(" enum") {
            Some((TagKeyword::Enum, auto_cleanup))
        } else {
            None
        }
    }
}
