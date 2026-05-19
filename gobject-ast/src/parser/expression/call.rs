use tree_sitter::Node;

use crate::{
    model::{AllocCallExpression, CallExpression, TypeInfo},
    parser::Parser,
};

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

    /// Parse g_allocation_call (g_new, g_new0, g_renew, etc.)
    /// These have g_allocation_argument_list with type as first arg, not
    /// expression.
    pub(crate) fn parse_g_allocation_call(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<AllocCallExpression> {
        let function_node = node.child_by_field_name("function")?;
        let function = self.parse_expression(function_node, source)?;

        let args_node = node.child_by_field_name("arguments")?;
        let mut cursor = args_node.walk();

        // Parse the type components
        let mut base_type = String::new();
        let mut pointer_depth = 0;
        let mut is_const = false;
        let mut arguments = Vec::new();

        for child in args_node.children(&mut cursor) {
            match child.kind() {
                "(" | ")" | "," => {
                    // Delimiters - skip
                }
                "type_qualifier" => {
                    let text = std::str::from_utf8(&source[child.byte_range()]).ok()?;
                    if text == "const" {
                        is_const = true;
                    }
                }
                "type_specifier" => {
                    // This is the base type (e.g., GList, Point, gchar)
                    base_type = std::str::from_utf8(&source[child.byte_range()])
                        .ok()?
                        .trim()
                        .to_owned();
                }
                "*" => {
                    pointer_depth += 1;
                }
                _ => {
                    // Any other named nodes are expression arguments
                    if Self::is_expression_node(&child)
                        && let Some(expr) = self.parse_expression(child, source)
                    {
                        arguments.push(Box::new(expr));
                    }
                }
            }
        }

        Some(AllocCallExpression {
            function: Box::new(function),
            allocated_type: TypeInfo {
                base_type,
                is_const,
                is_volatile: false,
                is_struct: false, // We don't track this in g_new calls
                is_union: false,
                pointer_depth,
                location: self.node_location(args_node),
                auto_cleanup: None,
            },
            arguments,
            location: self.node_location(node),
        })
    }
}
