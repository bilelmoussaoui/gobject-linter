use tree_sitter::Node;

use crate::{model::CallExpression, parser::Parser};

impl Parser {
    pub(crate) fn parse_call_expression(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<CallExpression> {
        let function_node = node.child_by_field_name("function")?;
        let function = self.parse_expression(function_node, source)?;

        let mut arguments = Vec::new();
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                if child.is_named()
                    && child.kind() != ","
                    && Self::is_expression_node(&child)
                    && let Some(expr) = self.parse_expression(child, source)
                {
                    arguments.push(Box::new(expr));
                }
            }
        }

        Some(CallExpression {
            function: Box::new(function),
            arguments,
            location: self.node_location(node),
        })
    }
}
