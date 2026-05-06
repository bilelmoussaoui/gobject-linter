use tree_sitter::Node;

use crate::{model::Expression, parser::Parser};

impl Parser {
    pub(crate) fn parse_expression_stmt(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<Box<Expression>> {
        // Get the actual expression inside the statement
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named()
                && child.kind() != ";"
                && Self::is_expression_node(&child)
                && let Some(expr) = self.parse_expression(child, source)
            {
                return Some(Box::new(expr));
            }
        }
        None
    }
}
