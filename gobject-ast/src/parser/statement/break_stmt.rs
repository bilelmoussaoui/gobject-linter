use tree_sitter::Node;

use crate::{model::statement::BreakStatement, parser::Parser};

impl Parser {
    pub(super) fn parse_break_statement(
        &self,
        node: Node,
        _source: &[u8],
    ) -> Option<BreakStatement> {
        Some(BreakStatement {
            location: self.node_location(node),
        })
    }
}
