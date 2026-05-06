use tree_sitter::Node;

use crate::{model::statement::ContinueStatement, parser::Parser};

impl Parser {
    pub(super) fn parse_continue_statement(
        &self,
        node: Node,
        _source: &[u8],
    ) -> Option<ContinueStatement> {
        Some(ContinueStatement {
            location: self.node_location(node),
        })
    }
}
