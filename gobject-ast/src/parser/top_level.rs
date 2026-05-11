use tree_sitter::Node;

use crate::{
    model::{
        Comment, CommentPosition, ConditionalKind, EnumInfo, EnumValue, Expression,
        FunctionDeclItem, FunctionDefItem, FunctionDoc, Parameter, PragmaKind,
        PreprocessorDirective, SourceLocation, Statement, StructField, TopLevelItem, TypeDefItem,
        TypeInfo, TypedefTarget,
    },
    parser::Parser,
};

impl Parser {
    /// Extract return type from a function declaration or definition
    pub(super) fn extract_return_type(&self, node: Node, source: &[u8]) -> TypeInfo {
        let mut cursor = node.walk();
        let mut type_node = None;
        let mut qualifiers: Vec<&str> = Vec::new();
        let mut declarator_node = None;

        // Find the type node by walking children
        // Now that grammar is fixed, macro_modifier will be a separate node we can skip
        for child in node.children(&mut cursor) {
            match child.kind() {
                "type_identifier" if type_node.is_none() => {
                    type_node = Some(child);
                }
                "primitive_type" | "sized_type_specifier" | "struct_specifier"
                    if type_node.is_none() =>
                {
                    type_node = Some(child);
                }
                "type_qualifier" => {
                    let text = std::str::from_utf8(&source[child.byte_range()]).unwrap_or("");
                    if matches!(text, "const" | "volatile") {
                        qualifiers.push(text);
                    }
                }
                "macro_modifier" => {}
                "pointer_declarator" | "function_declarator" => {
                    declarator_node = Some(child);
                    break;
                }
                _ => {}
            }
        }

        // Count pointer indirections: GList *foo() parses as
        // type_identifier("GList") > pointer_declarator(*) > function_declarator
        let pointer_depth = Self::count_declarator_pointers(declarator_node);

        // Extract type text
        let (full_type_text, start_byte, end_byte) = if let Some(type_n) = type_node {
            let text = std::str::from_utf8(&source[type_n.byte_range()]).unwrap_or("void");
            (text, type_n.start_byte(), type_n.end_byte())
        } else {
            ("void", node.start_byte(), node.start_byte())
        };

        let full_text = if qualifiers.is_empty() {
            full_type_text.to_owned()
        } else {
            format!("{} {}", qualifiers.join(" "), full_type_text)
        };

        let location = SourceLocation::new(
            node.start_position().row + 1,
            node.start_position().column + 1,
            start_byte,
            end_byte,
        );

        let mut type_info = TypeInfo::new(&full_text, location);
        type_info.pointer_depth += pointer_depth;
        type_info
    }

    /// Count pointer_declarator nesting depth before a function_declarator.
    fn count_declarator_pointers(node: Option<Node>) -> usize {
        let Some(n) = node else { return 0 };
        if n.kind() != "pointer_declarator" {
            return 0;
        }
        let mut depth = 0;
        let mut current = n;
        while current.kind() == "pointer_declarator" {
            depth += 1;
            let mut cursor = current.walk();
            let next = current
                .children(&mut cursor)
                .find(|c| c.kind() == "pointer_declarator" || c.kind() == "function_declarator");
            match next {
                Some(child) => current = child,
                None => break,
            }
        }
        depth
    }

    /// Parse a number literal string, handling both decimal and hexadecimal
    /// Returns None if the string cannot be parsed as a number
    fn parse_number_literal(literal: &str) -> Option<i64> {
        let trimmed = literal.trim();

        // Handle hex numbers (0x or 0X prefix)
        if let Some(hex_str) = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
        {
            return i64::from_str_radix(hex_str, 16).ok();
        }

        // Handle octal numbers (0 prefix, but not "0" alone)
        if trimmed.starts_with('0') && trimmed.len() > 1 && !trimmed.contains('.') {
            return i64::from_str_radix(&trimmed[1..], 8).ok();
        }

        // Handle decimal numbers
        trimmed.parse::<i64>().ok()
    }

    /// Find a function_declarator node within a declaration
    fn find_function_declarator_in_node<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        // Direct declarator field
        if let Some(declarator) = node.child_by_field_name("declarator")
            && let Some(func_decl) = self.find_function_declarator(declarator)
        {
            return Some(func_decl);
        }

        // Search all children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(func_decl) = self.find_function_declarator(child) {
                return Some(func_decl);
            }
        }

        None
    }

    /// Parse a top-level item (declaration, definition, preprocessor directive,
    /// etc.)
    #[tracing::instrument(skip(self, node, source), fields(node_kind = node.kind(), line = node.start_position().row + 1))]
    pub(super) fn parse_top_level_item(&self, node: Node, source: &[u8]) -> Option<TopLevelItem> {
        tracing::trace!("parse_top_level_item: {}", node.kind());
        match node.kind() {
            "preproc_include" => {
                let path_node = node.child_by_field_name("path")?;
                let path_text = std::str::from_utf8(&source[path_node.byte_range()]).ok()?;
                let is_system = path_text.starts_with('<');
                let path = path_text.trim_matches(&['<', '>', '"'][..]).to_owned();

                Some(TopLevelItem::Preprocessor(PreprocessorDirective::Include {
                    path,
                    is_system,
                    location: self.node_location(node),
                }))
            }
            "preproc_def" | "preproc_function_def" => {
                let name_node = node.child_by_field_name("name")?;
                let name = std::str::from_utf8(&source[name_node.byte_range()])
                    .ok()?
                    .to_owned();

                // Extract value if present (for #define FOO 123)
                let value = node.child_by_field_name("value").and_then(|value_node| {
                    std::str::from_utf8(&source[value_node.byte_range()])
                        .ok()
                        .map(std::borrow::ToOwned::to_owned)
                });

                Some(TopLevelItem::Preprocessor(PreprocessorDirective::Define {
                    name,
                    value,
                    location: self.node_location(node),
                }))
            }
            "preproc_call" => {
                let directive_node = node.child_by_field_name("directive")?;
                let directive = std::str::from_utf8(&source[directive_node.byte_range()])
                    .ok()?
                    .trim_start_matches('#')
                    .to_owned();

                // Parse #pragma directives specially
                if directive == "pragma" {
                    let arguments = node.child_by_field_name("argument").and_then(|arg_node| {
                        std::str::from_utf8(&source[arg_node.byte_range()])
                            .ok()
                            .map(|s| s.trim().to_owned())
                    });

                    let kind = self.parse_pragma_kind(&arguments);

                    return Some(TopLevelItem::Preprocessor(PreprocessorDirective::Pragma {
                        kind,
                        location: self.node_location(node),
                    }));
                }

                Some(TopLevelItem::Preprocessor(PreprocessorDirective::Call {
                    directive,
                    location: self.node_location(node),
                }))
            }
            "preproc_if" | "preproc_ifdef" | "preproc_ifndef" => {
                // Parse conditional preprocessor directives with their body
                // Note: tree-sitter-c uses "preproc_ifdef" for both #ifdef and #ifndef
                // We need to check the actual text to distinguish them
                let kind = if node.kind() == "preproc_ifdef" {
                    // Check if it's actually #ifndef by looking at the directive text
                    let first_child = node.child(0);
                    let is_ifndef = first_child
                        .and_then(|child| std::str::from_utf8(&source[child.byte_range()]).ok())
                        .is_some_and(|text| text == "#ifndef");

                    if is_ifndef {
                        ConditionalKind::Ifndef
                    } else {
                        ConditionalKind::Ifdef
                    }
                } else {
                    match node.kind() {
                        "preproc_ifndef" => ConditionalKind::Ifndef,
                        "preproc_if" => ConditionalKind::If,
                        _ => unreachable!(),
                    }
                };

                // Get condition (for #ifdef/#ifndef, it's the name; for #if, it's the whole
                // condition)
                let condition = if let Some(name_node) = node.child_by_field_name("name") {
                    Some(
                        std::str::from_utf8(&source[name_node.byte_range()])
                            .ok()?
                            .to_owned(),
                    )
                } else if let Some(cond_node) = node.child_by_field_name("condition") {
                    Some(
                        std::str::from_utf8(&source[cond_node.byte_range()])
                            .ok()?
                            .to_owned(),
                    )
                } else {
                    None
                };

                // Parse body items - recursively parse children that are not part of the
                // preprocessor syntax
                let body = self.parse_conditional_body(node, source);

                Some(TopLevelItem::Preprocessor(
                    PreprocessorDirective::Conditional {
                        kind,
                        condition,
                        body,
                        location: self.node_location(node),
                    },
                ))
            }
            "preproc_elif" => {
                let condition = node
                    .child_by_field_name("condition")
                    .and_then(|c| std::str::from_utf8(&source[c.byte_range()]).ok())
                    .map(std::borrow::ToOwned::to_owned);

                let body = self.parse_conditional_body(node, source);

                Some(TopLevelItem::Preprocessor(
                    PreprocessorDirective::Conditional {
                        kind: ConditionalKind::Elif,
                        condition,
                        body,
                        location: self.node_location(node),
                    },
                ))
            }
            "preproc_else" => {
                let body = self.parse_conditional_body(node, source);

                Some(TopLevelItem::Preprocessor(
                    PreprocessorDirective::Conditional {
                        kind: ConditionalKind::Else,
                        condition: None,
                        body,
                        location: self.node_location(node),
                    },
                ))
            }
            "gobject_decls_block" => {
                // Parse G_BEGIN_DECLS ... G_END_DECLS block
                let body = self.parse_conditional_body(node, source);

                Some(TopLevelItem::Preprocessor(
                    PreprocessorDirective::GObjectDeclsBlock {
                        body,
                        location: self.node_location(node),
                    },
                ))
            }
            "type_definition" => {
                // Check for typedef enum
                if let Some(enum_info) = self.extract_enum(node, source) {
                    return Some(TopLevelItem::TypeDefinition(TypeDefItem::Enum(Box::new(
                        enum_info,
                    ))));
                }
                if let Some(item) = self.extract_typedef_from_type_definition(node, source) {
                    return Some(TopLevelItem::TypeDefinition(item));
                }
                None
            }
            "declaration" => {
                tracing::debug!("Processing declaration node");

                // Check for enum declarations
                if let Some(enum_info) = self.extract_enum(node, source) {
                    return Some(TopLevelItem::TypeDefinition(TypeDefItem::Enum(Box::new(
                        enum_info,
                    ))));
                }

                // Check for a standalone struct definition: `struct _Foo { ... };`
                // This is a declaration whose first named child is a struct_specifier
                // with a body.  No typedef alias — just the struct itself.
                if let Some(struct_item) = self.try_parse_struct_definition(node, source) {
                    return Some(struct_item);
                }

                let func_declarator = self.find_function_declarator_in_node(node);

                if let Some(func_decl) = func_declarator {
                    // Skip function declarations that contain parse errors
                    if node.has_error() {
                        return None;
                    }
                    // Extract function name
                    if let Some(name) = self.extract_declarator_name(func_decl, source) {
                        tracing::debug!("Found function declarator with name: {}", name);

                        let decl_text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
                        let is_static = decl_text.contains("static");
                        let is_inline = decl_text.contains("inline");

                        // Extract export macros from first line
                        let export_macros = self.find_export_macros_in_declaration(node, source);

                        // Extract return type
                        let return_type = self.extract_return_type(node, source);

                        // Extract parameters from the function_declarator node
                        let parameters = {
                            let mut params = Vec::new();
                            let mut cursor = func_decl.walk();
                            if let Some(child) = func_decl
                                .children_by_field_name("parameters", &mut cursor)
                                .next()
                            {
                                params = self.extract_parameters(child, source);
                            }
                            if params.is_empty()
                                && let Some(params_node) =
                                    self.find_node_by_kind(func_decl, "parameter_list")
                            {
                                params = self.extract_parameters(params_node, source);
                            }
                            params
                        };

                        return Some(TopLevelItem::FunctionDeclaration(FunctionDeclItem {
                            name: name.to_owned(),
                            return_type,
                            is_static,
                            is_inline,
                            parameters,
                            export_macros,
                            location: self.node_location(node),
                            doc: FunctionDoc::from_node_for(node, source, name),
                        }));
                    }
                }

                // Variable or type declaration - parse as statement
                if let Some(stmt) = self.parse_statement(node, source) {
                    return match stmt {
                        Statement::Declaration(decl) => Some(TopLevelItem::Declaration(decl)),
                        Statement::Expression(expr) => Some(TopLevelItem::Expression(expr)),
                        _ => None,
                    };
                }
                None
            }
            "struct_specifier" | "union_specifier" => {
                // Standalone struct/union definition: `struct _Foo { ... };`
                // tree-sitter-c parses these as a bare struct_specifier node
                // (not wrapped in a declaration), with the semicolon as a
                // separate sibling.
                if let Some(body) = node.child_by_field_name("body") {
                    let name = node
                        .child_by_field_name("name")
                        .and_then(|n| std::str::from_utf8(&source[n.byte_range()]).ok())
                        .unwrap_or("")
                        .to_owned();

                    if !name.is_empty() {
                        let fields = self.extract_struct_fields_from_body(body, source);
                        let bare = name.trim_start_matches('_');
                        let vfuncs = if bare.ends_with("Class") || bare.ends_with("Interface") {
                            self.extract_vfuncs(body, source)
                        } else {
                            vec![]
                        };
                        return Some(TopLevelItem::TypeDefinition(TypeDefItem::Struct {
                            name,
                            fields,
                            vfuncs,
                            location: self.node_location(node),
                            doc: None,
                        }));
                    }
                }
                None
            }
            "enum_specifier" => {
                // Standalone enum (enum Name { ... } or anonymous enum { ... })
                if let Some(enum_info) = self.extract_enum(node, source) {
                    return Some(TopLevelItem::TypeDefinition(TypeDefItem::Enum(Box::new(
                        enum_info,
                    ))));
                }
                None
            }
            "function_definition" => {
                if self.is_bogus_function_definition(node) {
                    return None;
                }
                self.parse_function_definition_node(node, source)
            }
            "expression_statement" => self
                .parse_expression_stmt(node, source)
                .map(TopLevelItem::Expression),
            "gobject_type_macro" => {
                let full_text = std::str::from_utf8(&source[node.byte_range()]).unwrap_or("");
                let macro_name = full_text.split('(').next().unwrap_or("").trim();

                // Route cleanup-func macros before the generic G_DEFINE_ handler
                if macro_name == "G_DEFINE_AUTOPTR_CLEANUP_FUNC"
                    || macro_name == "G_DEFINE_AUTO_CLEANUP_CLEAR_FUNC"
                    || macro_name == "G_DEFINE_AUTO_CLEANUP_FREE_FUNC"
                {
                    let args_node = node
                        .children(&mut node.walk())
                        .find(|c| c.kind() == "argument_list");
                    if let Some(args_node) = args_node {
                        let mut args = Vec::new();
                        self.collect_identifiers(args_node, source, &mut args);
                        if args.len() >= 2 {
                            let directive = if macro_name == "G_DEFINE_AUTOPTR_CLEANUP_FUNC" {
                                PreprocessorDirective::AutoptrCleanupFunc {
                                    type_name: args[0].to_owned(),
                                    cleanup_function: args[1].to_owned(),
                                    location: self.node_location(node),
                                }
                            } else {
                                PreprocessorDirective::AutoCleanupClearFunc {
                                    type_name: args[0].to_owned(),
                                    cleanup_function: args[1].to_owned(),
                                    location: self.node_location(node),
                                }
                            };
                            return Some(TopLevelItem::Preprocessor(directive));
                        }
                    }
                    return None;
                }

                if let Some(gobject_type) = self.extract_gobject_from_macro_modifier(node, source) {
                    return Some(TopLevelItem::Preprocessor(
                        PreprocessorDirective::GObjectType(Box::new(gobject_type)),
                    ));
                }
                None
            }
            "comment" => {
                let text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
                let (kind, _) = self.extract_comment_text(node, source)?;
                Some(TopLevelItem::Comment(Comment::new(
                    text.to_string(),
                    self.node_location(node),
                    kind,
                    CommentPosition::Leading,
                )))
            }
            "ERROR" => {
                let snippet = std::str::from_utf8(&source[node.byte_range()])
                    .unwrap_or("<invalid utf8>")
                    .chars()
                    .take(80)
                    .collect::<String>();
                tracing::warn!(
                    "Unhandled ERROR node at {}:{} — fix the grammar. Content: {:?}",
                    node.start_position().row + 1,
                    node.start_position().column + 1,
                    snippet,
                );
                None
            }
            _ => None,
        }
    }

    /// A function_definition that contains nested function_definitions or
    /// gobject_type_macros is bogus which happens when a parse error (e.g.
    /// Objective-C `@interface`) causes tree-sitter to treat the rest of
    /// the file as a single function body.
    fn is_bogus_function_definition(&self, node: Node) -> bool {
        if !node.has_error() {
            return false;
        }
        let body = match node.child_by_field_name("body") {
            Some(b) => b,
            None => return false,
        };
        let mut cursor = body.walk();
        body.children(&mut cursor)
            .any(|c| matches!(c.kind(), "function_definition" | "gobject_type_macro"))
    }

    /// Parse a function definition from a node whose children include the
    /// return type, declarator, and body.  Works for both proper
    /// `function_definition` nodes and ERROR nodes with the same structure.
    fn parse_function_definition_node(&self, node: Node, source: &[u8]) -> Option<TopLevelItem> {
        let (name, is_static, is_inline) = self.extract_function_from_definition(node, source)?;

        let parameters = if let Some(declarator) = node.child_by_field_name("declarator") {
            let mut params = Vec::new();
            let mut cursor = declarator.walk();
            if let Some(child) = declarator
                .children_by_field_name("parameters", &mut cursor)
                .next()
            {
                params = self.extract_parameters(child, source);
            }
            if params.is_empty()
                && let Some(params_node) = self.find_node_by_kind(declarator, "parameter_list")
            {
                params = self.extract_parameters(params_node, source);
            }
            params
        } else if let Some(func_decl) = self.find_function_declarator(node) {
            if let Some(params_node) = self.find_node_by_kind(func_decl, "parameter_list") {
                self.extract_parameters(params_node, source)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let body = node.child_by_field_name("body");
        let (body_statements, body_location) = if let Some(b) = body {
            (
                self.parse_function_body(b, source),
                Some(self.node_location(b)),
            )
        } else {
            (
                self.collect_body_from_error_children(node, source),
                Some(self.node_location(node)),
            )
        };

        let return_type = self.extract_return_type(node, source);

        Some(TopLevelItem::FunctionDefinition(FunctionDefItem {
            name: name.to_owned(),
            return_type,
            is_static,
            is_inline,
            parameters,
            body_statements,
            location: self.node_location(node),
            body_location,
            doc: FunctionDoc::from_node_for(node, source, name),
        }))
    }

    /// Collect body statements from an ERROR node's flattened children
    /// (between `{` and `}`), skipping anything `parse_statement` doesn't
    /// recognize (preprocessor tokens, stray keywords, etc.).
    fn collect_body_from_error_children(&self, node: Node, source: &[u8]) -> Vec<Statement> {
        let mut statements = Vec::new();
        let mut cursor = node.walk();
        let mut inside_body = false;

        for child in node.children(&mut cursor) {
            if !child.is_named() && child.kind() == "{" {
                inside_body = true;
                continue;
            }
            if !child.is_named() && child.kind() == "}" {
                break;
            }
            if !inside_body || !child.is_named() {
                continue;
            }
            if let Some(stmt) = self.parse_statement(child, source) {
                statements.push(stmt);
            }
        }

        statements
    }

    /// Recover a function definition from an ERROR node.
    ///
    /// When `#ifdef` / `#endif` appear inside a function body (e.g. wrapping
    /// an `else if` branch), tree-sitter cannot assemble the surrounding
    /// `function_definition` and produces an ERROR node whose children are the
    /// same as a normal function_definition, just not wrapped in one.
    pub(super) fn try_recover_function_from_error(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<TopLevelItem> {
        self.find_function_declarator(node)?;

        let mut cursor = node.walk();
        let has_open_brace = node
            .children(&mut cursor)
            .any(|c| !c.is_named() && c.kind() == "{");
        if !has_open_brace {
            return None;
        }

        let item = self.parse_function_definition_node(node, source)?;

        if let TopLevelItem::FunctionDefinition(ref f) = item {
            tracing::info!(
                "Recovered function '{}' from ERROR node at {}:{}",
                f.name,
                node.start_position().row + 1,
                node.start_position().column + 1,
            );
        }

        Some(item)
    }

    /// If `declaration_node` is a *pure* struct definition (`struct _Foo { …
    /// };`), produce a `TypeDefItem::Struct` with parsed fields.
    ///
    /// Returns `None` for:
    /// - Anonymous inline structs used as a variable type: `static const struct
    ///   { … } arr[];`
    /// - Forward declarations without a body: `struct _Foo;`
    /// - Declarations that also declare a variable: `struct _Foo { … } var;`
    fn try_parse_struct_definition(
        &self,
        declaration_node: Node,
        source: &[u8],
    ) -> Option<TopLevelItem> {
        // If the declaration also declares a variable (has a `declarator` field
        // like `struct _Foo { … } var;`), let it fall through to parse_statement
        // so the variable declaration is not lost.
        if declaration_node.child_by_field_name("declarator").is_some() {
            return None;
        }

        let mut cursor = declaration_node.walk();
        for child in declaration_node.children(&mut cursor) {
            if matches!(child.kind(), "struct_specifier" | "union_specifier")
                && let Some(body) = child.child_by_field_name("body")
            {
                let name = child
                    .child_by_field_name("name")
                    .and_then(|n| std::str::from_utf8(&source[n.byte_range()]).ok())
                    .unwrap_or("")
                    .to_owned();

                // Skip anonymous structs (e.g. `static const struct { … } arr[];`).
                // We already checked for declarator above, but an anonymous struct
                // with no declarator is an unusual edge case — skip it too.
                if name.is_empty() {
                    return None;
                }

                let fields = self.extract_struct_fields_from_body(body, source);
                let bare = name.trim_start_matches('_');
                let vfuncs = if bare.ends_with("Class") || bare.ends_with("Interface") {
                    self.extract_vfuncs(body, source)
                } else {
                    vec![]
                };

                return Some(TopLevelItem::TypeDefinition(TypeDefItem::Struct {
                    name,
                    fields,
                    vfuncs,
                    location: self.node_location(declaration_node),
                    doc: None,
                }));
            }
        }
        None
    }

    /// Extract field declarations from a `field_declaration_list` tree-sitter
    /// node.
    fn extract_struct_fields_from_body(&self, body: Node, source: &[u8]) -> Vec<StructField> {
        let mut fields = Vec::new();
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            match child.kind() {
                "field_declaration" => {
                    let Some(type_node) = child.child_by_field_name("type") else {
                        continue;
                    };

                    // Anonymous struct/union: store as a field with inner_fields
                    // so callers can see `union { A a; B b; } d` as field d with
                    // inner fields [a, b], preserving the aggregate structure.
                    if matches!(type_node.kind(), "struct_specifier" | "union_specifier")
                        && type_node.child_by_field_name("name").is_none()
                    {
                        let inner_fields = type_node
                            .child_by_field_name("body")
                            .map(|b| self.extract_struct_fields_from_body(b, source))
                            .unwrap_or_default();
                        let field_name = child
                            .child_by_field_name("declarator")
                            .and_then(|d| self.extract_field_declarator_name(d, source))
                            .map(std::borrow::ToOwned::to_owned);
                        fields.push(StructField {
                            field_type: TypeInfo::new("", self.node_location(type_node)),
                            field_name,
                            location: self.node_location(child),
                            bit_width: None,
                            inner_fields,
                        });
                        continue;
                    }

                    let type_text = match type_node.kind() {
                        "type_identifier" | "primitive_type" | "sized_type_specifier" => {
                            std::str::from_utf8(&source[type_node.byte_range()])
                                .ok()
                                .map(str::trim)
                        }
                        "struct_specifier" | "union_specifier" | "enum_specifier" => {
                            // Named tag: grab the name so type_references tracks it.
                            type_node
                                .child_by_field_name("name")
                                .and_then(|n| std::str::from_utf8(&source[n.byte_range()]).ok())
                        }
                        _ => None,
                    };

                    let Some(text) = type_text else { continue };
                    if text.is_empty() {
                        continue;
                    }

                    let field_type = TypeInfo::new(text, self.node_location(type_node));

                    let field_name = child
                        .child_by_field_name("declarator")
                        .and_then(|d| self.extract_field_declarator_name(d, source))
                        .map(std::borrow::ToOwned::to_owned);

                    let bit_width = {
                        let mut cursor = child.walk();
                        child
                            .children(&mut cursor)
                            .find(|c| c.kind() == "bitfield_clause")
                            .and_then(|bc| bc.named_child(0))
                            .and_then(|w| std::str::from_utf8(&source[w.byte_range()]).ok())
                            .and_then(|s| s.trim().parse::<u32>().ok())
                    };

                    fields.push(StructField {
                        field_type,
                        field_name,
                        location: self.node_location(child),
                        bit_width,
                        inner_fields: vec![],
                    });
                }
                // Recurse into nested anonymous struct/union bodies
                "field_declaration_list" => {
                    fields.extend(self.extract_struct_fields_from_body(child, source));
                }
                _ => {}
            }
        }

        fields
    }

    /// Like `extract_declarator_name` but also handles `field_identifier`
    /// (used in struct field declarators).
    fn extract_field_declarator_name<'a>(
        &self,
        declarator: Node,
        source: &'a [u8],
    ) -> Option<&'a str> {
        if matches!(declarator.kind(), "field_identifier" | "identifier") {
            return std::str::from_utf8(&source[declarator.byte_range()]).ok();
        }
        if let Some(inner) = declarator.child_by_field_name("declarator") {
            return self.extract_field_declarator_name(inner, source);
        }
        None
    }

    pub(super) fn extract_typedef_from_type_definition(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<TypeDefItem> {
        // type_definition has "declarator" for the typedef name and "type" for what
        // it's typedef'ing
        let declarator_node = node.child_by_field_name("declarator")?;

        // For simple typedefs (`typedef Struct Name`), the declarator IS the name.
        // For function-pointer typedefs (`typedef RetType (*Name)(params)`), the
        // declarator is a function_declarator tree — drill into it to get just the
        // identifier.  Same for array typedefs (`typedef int Name[N]`).
        let name = if matches!(declarator_node.kind(), "type_identifier" | "identifier") {
            std::str::from_utf8(&source[declarator_node.byte_range()])
                .ok()?
                .to_owned()
        } else {
            self.extract_declarator_name(declarator_node, source)?
                .to_owned()
        };

        // When the typedef wraps an inline struct body, extract field declarations
        // so rules can see which types are referenced inside the struct.
        let struct_fields = node
            .child_by_field_name("type")
            .filter(|n| matches!(n.kind(), "struct_specifier" | "union_specifier"))
            .and_then(|s| s.child_by_field_name("body"))
            .map(|body| self.extract_struct_fields_from_body(body, source))
            .unwrap_or_default();

        // Detect function-pointer typedefs: `typedef RetType (*Name)(params)`.
        // The declarator will contain a function_declarator node.
        let target = if let Some(func_decl) = self.find_function_declarator(declarator_node) {
            let return_type = self.extract_return_type(node, source);

            let mut parameters = func_decl
                .children_by_field_name("parameters", &mut func_decl.walk())
                .next()
                .map(|p| self.extract_parameters(p, source))
                .unwrap_or_default();
            if parameters.is_empty()
                && let Some(params_node) = self.find_node_by_kind(func_decl, "parameter_list")
            {
                parameters = self.extract_parameters(params_node, source);
            }
            TypedefTarget::Callback {
                return_type,
                parameters,
            }
        } else {
            let type_node = node.child_by_field_name("type")?;
            let target_text = std::str::from_utf8(&source[type_node.byte_range()]).ok()?;
            let target_type = TypeInfo::new(target_text, self.node_location(type_node));
            TypedefTarget::Type(target_type)
        };

        Some(TypeDefItem::Typedef {
            name,
            target,
            struct_fields,
            location: self.node_location(node),
            doc: None,
        })
    }

    pub(super) fn extract_enum(&self, node: Node, source: &[u8]) -> Option<EnumInfo> {
        // Check if this is a typedef or regular declaration containing an enum
        let node_text = std::str::from_utf8(&source[node.byte_range()]).ok()?;
        if !node_text.contains("enum") {
            return None;
        }

        // Handle direct enum_specifier node
        if node.kind() == "enum_specifier"
            && let Some(body) = node.child_by_field_name("body")
        {
            let values = self.extract_enum_values(body, source);

            // Try to get the name from the name field
            let name = node.child_by_field_name("name").and_then(|name_node| {
                std::str::from_utf8(&source[name_node.byte_range()])
                    .ok()
                    .map(std::borrow::ToOwned::to_owned)
            });

            return Some(EnumInfo {
                name,
                location: self.node_location(node),
                values,
                body_location: self.node_location(body),
                attributes: Vec::new(),
                doc: None,
            });
        }

        // Handle typedef enum { ... } Name;
        if node.kind() == "type_definition" {
            if let Some(type_node) = node.child_by_field_name("type")
                && type_node.kind() == "enum_specifier"
                && let Some(body) = type_node.child_by_field_name("body")
            {
                // Collect type_identifiers (attribute macros like G_GNUC_FLAG_ENUM)
                // and the actual typedef name from type_definition children.
                // When a macro attribute precedes the name, tree-sitter records
                // the macro as type_identifier and the actual name as an ERROR node.
                let mut attributes = Vec::new();
                let mut error_name: Option<String> = None;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "type_identifier" {
                        if let Ok(text) = std::str::from_utf8(&source[child.byte_range()]) {
                            attributes.push(text.to_owned());
                        }
                    } else if child.kind() == "ERROR" {
                        // Without includes the grammar records the real typedef name as an ERROR
                        // node inside the type_definition.
                        error_name = std::str::from_utf8(&source[child.byte_range()])
                            .ok()
                            .map(|s| s.trim().to_owned())
                            .filter(|s| !s.is_empty());
                    }
                }

                // The name placement varies by parse context; try in order.
                let name = if error_name.is_some() {
                    error_name
                } else if let Some(next) = node.next_sibling() {
                    if next.kind() == "type_identifier" {
                        std::str::from_utf8(&source[next.byte_range()])
                            .ok()
                            .map(std::borrow::ToOwned::to_owned)
                    } else if next.kind() == "expression_statement" {
                        std::str::from_utf8(&source[next.byte_range()])
                            .ok()
                            .map(|s| s.trim().trim_end_matches(';').trim().to_owned())
                            .filter(|s| !s.is_empty() && !s.contains(' '))
                    } else {
                        attributes.pop()
                    }
                } else {
                    attributes.pop()
                };

                let values = self.extract_enum_values(body, source);
                return Some(EnumInfo {
                    name,
                    location: self.node_location(node),
                    values,
                    body_location: self.node_location(body),
                    attributes,
                    doc: None,
                });
            }
            return None;
        }

        // Handle standalone enum Name { ... }; or anonymous enum { ... }; - parse as
        // declaration first
        if let Some(Statement::Declaration(_)) = self.parse_statement(node, source) {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "enum_specifier"
                    && let Some(body) = child.child_by_field_name("body")
                {
                    let values = self.extract_enum_values(body, source);

                    // Try to get the name from the name field
                    let name = child.child_by_field_name("name").and_then(|name_node| {
                        std::str::from_utf8(&source[name_node.byte_range()])
                            .ok()
                            .map(std::borrow::ToOwned::to_owned)
                    });

                    return Some(EnumInfo {
                        name,
                        location: self.node_location(child),
                        values,
                        body_location: self.node_location(body),
                        attributes: Vec::new(),
                        doc: None,
                    });
                }
            }
        }
        None
    }

    pub(super) fn extract_enum_values(&self, body_node: Node, source: &[u8]) -> Vec<EnumValue> {
        let mut values = Vec::new();

        let mut cursor = body_node.walk();
        for child in body_node.children(&mut cursor) {
            if child.kind() == "enumerator"
                && let Some(name_node) = child.child_by_field_name("name")
            {
                let name = std::str::from_utf8(&source[name_node.byte_range()])
                    .unwrap_or("")
                    .to_owned();

                let (value, value_expr, value_location) =
                    if let Some(value_node) = child.child_by_field_name("value") {
                        // Parse as expression (only if it's actually an expression node)
                        let expr = if Self::is_expression_node(&value_node) {
                            self.parse_expression(value_node, source)
                        } else {
                            None
                        };

                        let parsed_value = expr.as_ref().and_then(|e| match e {
                            Expression::NumberLiteral(n) => Self::parse_number_literal(&n.value),
                            Expression::Identifier(_) => None, // Symbolic constant
                            _ => None,
                        });

                        (parsed_value, expr, Some(self.node_location(value_node)))
                    } else {
                        (None, None, None)
                    };

                let export_macros = self.find_export_macros_in_declaration(child, source);

                values.push(EnumValue {
                    name,
                    value,
                    value_expr,
                    location: self.node_location(child),
                    name_location: self.node_location(name_node),
                    value_location,
                    export_macros,
                    doc: None,
                });
            }
        }

        values
    }

    fn find_node_by_kind<'a>(&self, node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_node_by_kind(child, kind) {
                return Some(found);
            }
        }
        None
    }

    pub(super) fn extract_parameters(&self, params_node: Node, source: &[u8]) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if !child.is_named() {
                continue;
            }

            if child.kind() == "variadic_parameter" {
                parameters.push(Parameter::Variadic);
                continue;
            }

            if child.kind() != "parameter_declaration" {
                continue;
            }

            let type_node = child.child_by_field_name("type");
            let base_type = type_node
                .and_then(|t| std::str::from_utf8(&source[t.byte_range()]).ok())
                .unwrap_or_default()
                .to_owned();

            let declarator = child.child_by_field_name("declarator");
            let name = declarator
                .as_ref()
                .and_then(|d| self.extract_declarator_name(*d, source));

            // Count pointer levels from declarator
            let pointer_depth = if let Some(decl) = declarator {
                self.count_pointer_levels(decl)
            } else {
                0
            };

            // Build full type text
            let mut full_text = base_type;
            if pointer_depth > 0 {
                full_text.push_str(&"*".repeat(pointer_depth));
            }

            // Use type node's location if available
            let param_location = type_node
                .map(|node| self.node_location(node))
                .unwrap_or_default();
            let type_info = TypeInfo::new(&full_text, param_location);

            parameters.push(Parameter::Regular {
                name: name.map(ToOwned::to_owned),
                type_info,
                location: self.node_location(child),
            });
        }

        parameters
    }

    fn count_pointer_levels(&self, node: Node) -> usize {
        let mut count = 0;
        let mut current = node;

        loop {
            if current.kind() == "pointer_declarator"
                || current.kind() == "abstract_pointer_declarator"
            {
                count += 1;
                if let Some(inner) = current.child_by_field_name("declarator") {
                    current = inner;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        count
    }

    pub(super) fn extract_declarator_name<'a>(
        &self,
        declarator: Node,
        source: &'a [u8],
    ) -> Option<&'a str> {
        if let Some(inner) = declarator.child_by_field_name("declarator") {
            if matches!(inner.kind(), "identifier" | "type_identifier") {
                let text = std::str::from_utf8(&source[inner.byte_range()]).ok()?;
                // __attribute__ appearing between '*' and the real name means the
                // function_declarator's "declarator" field points at the attribute
                // keyword rather than the actual identifier.  Recover by searching
                // sibling call_expression nodes for the real name.
                if text == "__attribute__" {
                    let mut cursor = declarator.walk();
                    for child in declarator.children(&mut cursor) {
                        if child.kind() == "call_expression"
                            && let Some(func_node) = child.child_by_field_name("function")
                            && func_node.kind() == "identifier"
                        {
                            return std::str::from_utf8(&source[func_node.byte_range()]).ok();
                        }
                    }
                    return None;
                }
                return Some(text);
            }
            return self.extract_declarator_name(inner, source);
        }

        if matches!(declarator.kind(), "identifier" | "type_identifier") {
            let name = &source[declarator.byte_range()];
            return std::str::from_utf8(name).ok();
        }

        // Handle parenthesized declarators: `(function_name)` to prevent macro
        // expansion, or `(*Name)` inside function-pointer typedefs.
        if declarator.kind() == "parenthesized_declarator" {
            let mut cursor = declarator.walk();
            for child in declarator.children(&mut cursor) {
                if matches!(child.kind(), "identifier" | "type_identifier") {
                    let name = &source[child.byte_range()];
                    return std::str::from_utf8(name).ok();
                }
                if child.is_named()
                    && let Some(name) = self.extract_declarator_name(child, source)
                {
                    return Some(name);
                }
            }
        }

        None
    }

    /// Parse pragma arguments into a PragmaKind
    fn parse_pragma_kind(&self, arguments: &Option<String>) -> PragmaKind {
        let Some(args) = arguments else {
            return PragmaKind::Other {
                name: String::new(),
                arguments: None,
            };
        };

        // Check for "once"
        if args == "once" {
            return PragmaKind::Once;
        }

        // Check for diagnostic directives
        // Formats: "GCC diagnostic push", "clang diagnostic push", etc.
        if args.contains("diagnostic") {
            if args.contains("push") {
                return PragmaKind::DiagnosticPush;
            }
            if args.contains("pop") {
                return PragmaKind::DiagnosticPop;
            }
            // Check for "diagnostic ignored"
            if args.contains("ignored") {
                // Extract warning name from quotes
                // Format: "GCC diagnostic ignored \"-Wwarning-name\""
                if let Some(start) = args.find('"')
                    && let Some(end) = args[start + 1..].find('"')
                {
                    let warning = args[start + 1..start + 1 + end].to_string();
                    return PragmaKind::DiagnosticIgnored { warning };
                }
            }
        }

        // Everything else goes to Other
        // Split into name and arguments
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let name = parts[0].to_string();
        let arguments = parts.get(1).map(std::string::ToString::to_string);

        PragmaKind::Other { name, arguments }
    }

    /// Parse the body of a conditional preprocessor block (#ifdef, #if, etc.)
    pub(super) fn parse_conditional_body(&self, node: Node, source: &[u8]) -> Vec<TopLevelItem> {
        let mut body = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            // Skip preprocessor markers (#ifdef, #endif, etc.)
            if !child.is_named()
                || matches!(
                    child.kind(),
                    "#ifdef" | "#ifndef" | "#if" | "#elif" | "#else" | "#endif"
                )
            {
                continue;
            }

            if let Some(item) = self.parse_top_level_item(child, source) {
                body.push(item);
            }
        }

        body
    }
}
