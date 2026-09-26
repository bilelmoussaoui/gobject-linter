use std::sync::Arc;

use tree_sitter::Node;

use crate::{
    model::{Expression, SourceLocation, TypeInfo, VariableDecl},
    parser::Parser,
};

impl Parser {
    pub(crate) fn parse_variable_decl(&self, node: Node, source: &[u8]) -> Option<VariableDecl> {
        self.parse_variable_decls(node, source).into_iter().next()
    }

    pub(crate) fn parse_variable_decls(&self, node: Node, source: &[u8]) -> Vec<VariableDecl> {
        // declaration contains type specifiers (shared) and one or more
        // declarators
        let mut type_parts = Vec::new();
        let mut declarators = Vec::new();
        let mut first_type_node: Option<Node> = None;
        let mut last_type_node: Option<Node> = None;
        let mut is_static = false;
        let mut macro_modifier_name: Option<&str> = None;
        let mut macro_modifier_location: Option<SourceLocation> = None;
        let mut error_nodes: Vec<Node> = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_qualifier" => {
                    let Some(qualifier) = std::str::from_utf8(&source[child.byte_range()]).ok()
                    else {
                        continue;
                    };
                    type_parts.push(qualifier);
                    if first_type_node.is_none() {
                        first_type_node = Some(child);
                    }
                    last_type_node = Some(child);
                }
                "storage_class_specifier" => {
                    let Some(text) = std::str::from_utf8(&source[child.byte_range()]).ok() else {
                        continue;
                    };
                    if text == "static" {
                        is_static = true;
                    }
                    type_parts.push(text);
                    if first_type_node.is_none() {
                        first_type_node = Some(child);
                    }
                    last_type_node = Some(child);
                }
                "type_specifier"
                | "type_identifier"
                | "primitive_type"
                | "sized_type_specifier"
                | "struct_specifier"
                | "macro_type_specifier" => {
                    let Some(text) = std::str::from_utf8(&source[child.byte_range()]).ok() else {
                        continue;
                    };
                    type_parts.push(text);
                    if first_type_node.is_none() {
                        first_type_node = Some(child);
                    }
                    last_type_node = Some(child);
                }
                "macro_modifier" => {
                    if let Ok(text) = std::str::from_utf8(&source[child.byte_range()]) {
                        macro_modifier_name = Some(text);
                        macro_modifier_location = Some(self.node_location(child));
                    }
                }
                "init_declarator" => {
                    declarators.push(child);
                }
                "pointer_declarator" | "identifier" | "array_declarator"
                    if declarators.is_empty() =>
                {
                    declarators.push(child);
                }
                "ERROR" => {
                    error_nodes.push(child);
                }
                _ => {}
            }
        }

        if declarators.is_empty() {
            return Vec::new();
        }

        let first_type = match first_type_node {
            Some(n) => n,
            None => return Vec::new(),
        };
        let last_type = match last_type_node {
            Some(n) => n,
            None => return Vec::new(),
        };

        let num_declarators = declarators.len();
        let mut results = Vec::with_capacity(num_declarators);

        for (i, mut declarator) in declarators.into_iter().enumerate() {
            let mut type_parts = type_parts.clone();

            // g_autofd fixup only applies to the first (single) declarator
            if i == 0
                && declarator.kind() == "identifier"
                && !error_nodes.is_empty()
                && let Ok(text) = std::str::from_utf8(&source[declarator.byte_range()])
                && TypeInfo::new(text, SourceLocation::default())
                    .as_basic()
                    .is_some()
                && TypeInfo::parse_auto_cleanup(&type_parts.join(" ")).is_some()
            {
                type_parts.push(text);
                if let Some(err_node) = error_nodes.first() {
                    declarator = *err_node;
                }
            }

            let mut var_name = None;
            let mut var_name_location: Option<SourceLocation> = None;
            let mut initializer = None;

            let Some(declarator_text) = std::str::from_utf8(&source[declarator.byte_range()]).ok()
            else {
                continue;
            };
            let pointer_depth = declarator_text.chars().filter(|&c| c == '*').count();

            let array_size = self.extract_array_size(declarator, source);

            let mut dec_cursor = declarator.walk();
            let mut has_equals = false;
            for child in declarator.children(&mut dec_cursor) {
                if child.kind() == "=" {
                    has_equals = true;
                    continue;
                }

                if !has_equals {
                    match child.kind() {
                        "pointer_declarator"
                        | "identifier"
                        | "array_declarator"
                        | "parenthesized_declarator" => {
                            if let Some((id, loc)) =
                                self.find_identifier_with_location(child, source)
                            {
                                var_name = Some(id);
                                var_name_location = Some(loc);
                            }
                        }
                        _ => {}
                    }
                } else if child.is_named() && Self::is_expression_node(&child) {
                    initializer = self.parse_expression(child, source);
                }
            }

            if var_name.is_none_or(str::is_empty)
                && let Some(name) = macro_modifier_name
            {
                var_name = Some(name);
                var_name_location = macro_modifier_location.clone();
            }

            // When the declarator is a bare identifier leaf its .children() is
            // empty so the loop above never ran.
            if var_name.is_none() && declarator.kind() == "identifier" {
                let auto = TypeInfo::parse_auto_cleanup(&type_parts.join(" "));
                if auto.as_ref().is_some_and(|m| m.type_arg().is_none()) {
                    type_parts.push(declarator_text);
                    var_name = Some("");
                } else {
                    var_name = Some(declarator_text);
                    var_name_location = Some(self.node_location(declarator));
                }
            }

            let mut full_text = type_parts.join(" ");
            if pointer_depth > 0 {
                full_text.push_str(&"*".repeat(pointer_depth));
            }

            let type_location = SourceLocation::new(
                first_type.start_position().row + 1,
                first_type.start_position().column + 1,
                first_type.start_byte(),
                last_type.end_byte(),
                Arc::clone(&self.current_source),
            );
            let type_info = TypeInfo::new(&full_text, type_location);

            let Some(name) = var_name else { continue };
            let Some(name_location) = var_name_location else {
                continue;
            };

            results.push(VariableDecl {
                type_info,
                name: name.to_owned(),
                is_static,
                name_location,
                initializer,
                array_size,
                location: if num_declarators > 1 {
                    self.node_location(declarator)
                } else {
                    self.node_location(node)
                },
            });
        }

        results
    }

    /// Find identifier and its location in the source
    pub(super) fn find_identifier_with_location<'a>(
        &self,
        node: Node,
        source: &'a [u8],
    ) -> Option<(&'a str, SourceLocation)> {
        if node.kind() == "identifier" {
            let text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
            let location = self.node_location(node);
            return Some((text, location));
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(result) = self.find_identifier_with_location(child, source) {
                return Some(result);
            }
        }

        None
    }

    /// Extract array size expression from a declarator (recursively searches
    /// for array_declarator) e.g., for "int arr[N_PROPS]", extracts N_PROPS
    /// as an expression
    pub(super) fn extract_array_size(&self, declarator: Node, source: &[u8]) -> Option<Expression> {
        // Recursively find array_declarator and extract its size
        self.find_array_size_recursive(declarator, source)
    }

    fn find_array_size_recursive(&self, node: Node, source: &[u8]) -> Option<Expression> {
        if node.kind() == "array_declarator" {
            let mut cursor = node.walk();
            let mut found_bracket = false;
            for child in node.children(&mut cursor) {
                // Skip everything until we find "["
                if child.kind() == "[" {
                    found_bracket = true;
                    continue;
                }
                // Stop at "]"
                if child.kind() == "]" {
                    break;
                }
                // After "[", look for the size expression
                if found_bracket && child.is_named() && Self::is_expression_node(&child) {
                    return self.parse_expression(child, source);
                }
            }
            return None;
        }

        // Recursively search children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(size) = self.find_array_size_recursive(child, source) {
                return Some(size);
            }
        }

        None
    }
}
