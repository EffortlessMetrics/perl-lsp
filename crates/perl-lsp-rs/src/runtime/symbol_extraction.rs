//! AST-based symbol extraction and reference counting.
//!
//! These methods walk AST trees to extract workspace symbols or count
//! references to a given symbol. They are used by code-lens resolve,
//! workspace/symbol, and related features.

use super::*;

#[allow(dead_code)]
impl LspServer {
    /// Extract workspace symbols from a document's AST
    #[cfg(feature = "workspace")]
    pub(crate) fn extract_document_symbols(
        &self,
        ast: &perl_parser::ast::Node,
        source: &str,
        uri: &str,
    ) -> Vec<LspWorkspaceSymbol> {
        let mut symbols = Vec::new();
        self.extract_symbols_recursive(ast, source, uri, None, &mut symbols);
        symbols
    }

    #[cfg(not(feature = "workspace"))]
    pub(crate) fn extract_document_symbols(
        &self,
        _ast: &perl_parser::ast::Node,
        _source: &str,
        _uri: &str,
    ) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// Recursively extract symbols from an AST node
    #[cfg(feature = "workspace")]
    fn extract_symbols_recursive(
        &self,
        node: &perl_parser::ast::Node,
        source: &str,
        uri: &str,
        container: Option<&str>,
        symbols: &mut Vec<LspWorkspaceSymbol>,
    ) {
        use perl_ast::classification::NodeKindCategory;
        use perl_parser::ast::NodeKind;

        match &node.kind {
            // ── Drift-guard: all Declaration variants are funnelled through this arm.
            //
            // The outer guard `kind.category() == NodeKindCategory::Declaration` ensures
            // that any new NodeKind variant added to the Declaration category MUST be
            // handled in the inner match below — a missing arm is a compile error.
            // This prevents silent symbol drops when the parser gains new declaration
            // constructs (e.g. a future NodeKind::Role or NodeKind::Trait).
            //
            // Note: FunctionCall{name=="has"} is Expression-category, not Declaration,
            // so Moo/Moose `has` attributes are handled in their own arm below.
            kind if kind.category() == NodeKindCategory::Declaration => {
                match &node.kind {
                    NodeKind::Subroutine { name, body, .. } => {
                        // Add the subroutine as a symbol if it has a name
                        if let Some(sub_name) = name {
                            let (start_line, start_char) =
                                byte_to_line_col(source, node.location.start);
                            let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                            symbols.push(LspWorkspaceSymbol {
                                name: sub_name.clone(),
                                kind: 12, // Function
                                location: WireLocation::new(
                                    uri.to_string(),
                                    WireRange::new(
                                        WirePosition::new(start_line, start_char),
                                        WirePosition::new(end_line, end_char),
                                    ),
                                ),
                                container_name: container
                                    .map(|s| normalize_package_separator(s).into_owned()),
                                workspace_folder_uri: None,
                            });

                            // Recurse into body with this subroutine as container
                            self.extract_symbols_recursive(
                                body,
                                source,
                                uri,
                                Some(sub_name.as_str()),
                                symbols,
                            );
                        }
                    }

                    NodeKind::Package { name, block, .. } => {
                        // Add the package as a symbol
                        let (start_line, start_char) =
                            byte_to_line_col(source, node.location.start);
                        let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                        symbols.push(LspWorkspaceSymbol {
                            name: name.clone(),
                            kind: 2, // Module
                            location: WireLocation::new(
                                uri.to_string(),
                                WireRange::new(
                                    WirePosition::new(start_line, start_char),
                                    WirePosition::new(end_line, end_char),
                                ),
                            ),
                            container_name: container
                                .map(|s| normalize_package_separator(s).into_owned()),
                            workspace_folder_uri: None,
                        });

                        // Recurse into block with this package as container
                        if let Some(block) = block {
                            self.extract_symbols_recursive(
                                block,
                                source,
                                uri,
                                Some(name.as_str()),
                                symbols,
                            );
                        }
                    }

                    // Perl 5.38+ native class declaration
                    NodeKind::Class { name, body, .. } => {
                        let (start_line, start_char) =
                            byte_to_line_col(source, node.location.start);
                        let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                        symbols.push(LspWorkspaceSymbol {
                            name: name.clone(),
                            kind: 5, // Class
                            location: WireLocation::new(
                                uri.to_string(),
                                WireRange::new(
                                    WirePosition::new(start_line, start_char),
                                    WirePosition::new(end_line, end_char),
                                ),
                            ),
                            container_name: container
                                .map(|s| normalize_package_separator(s).into_owned()),
                            workspace_folder_uri: None,
                        });

                        // Recurse into body with this class as container
                        self.extract_symbols_recursive(
                            body,
                            source,
                            uri,
                            Some(name.as_str()),
                            symbols,
                        );
                    }

                    // Perl 5.38+ native method declaration
                    NodeKind::Method { name, body, .. } => {
                        let (start_line, start_char) =
                            byte_to_line_col(source, node.location.start);
                        let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                        symbols.push(LspWorkspaceSymbol {
                            name: name.clone(),
                            kind: 6, // Method
                            location: WireLocation::new(
                                uri.to_string(),
                                WireRange::new(
                                    WirePosition::new(start_line, start_char),
                                    WirePosition::new(end_line, end_char),
                                ),
                            ),
                            container_name: container
                                .map(|s| normalize_package_separator(s).into_owned()),
                            workspace_folder_uri: None,
                        });

                        // Recurse into body with this method as container
                        self.extract_symbols_recursive(
                            body,
                            source,
                            uri,
                            Some(name.as_str()),
                            symbols,
                        );
                    }

                    // `our` package-interface variables — index with sigil-prefixed name.
                    // `my` / `local` / `state` are sub-local and must NOT appear in the outline.
                    NodeKind::VariableDeclaration { declarator, variable, .. }
                        if declarator == "our" =>
                    {
                        if let NodeKind::Variable { sigil, name } = &variable.kind {
                            let display_name = format!("{sigil}{name}");
                            let (start_line, start_char) =
                                byte_to_line_col(source, node.location.start);
                            let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                            symbols.push(LspWorkspaceSymbol {
                                name: display_name,
                                kind: 13, // Variable
                                location: WireLocation::new(
                                    uri.to_string(),
                                    WireRange::new(
                                        WirePosition::new(start_line, start_char),
                                        WirePosition::new(end_line, end_char),
                                    ),
                                ),
                                container_name: container
                                    .map(|s| normalize_package_separator(s).into_owned()),
                                workspace_folder_uri: None,
                            });
                        }
                    }

                    // All other Declaration variants (Use, No, PhaseBlock, DataSection,
                    // Format, VariableListDeclaration, Prototype, Signature, *Parameter, etc.)
                    // are not indexed as workspace symbols. A future new Declaration variant
                    // will cause a compile error above unless this wildcard covers it, which
                    // forces an explicit decision: emit a symbol or leave it here.
                    _ => {}
                }
            }

            // Moo/Moose `has 'attr' => (...)` declarations.
            // FunctionCall is Expression-category (NOT Declaration), so it is handled
            // as its own outer arm — outside the Declaration drift-guard above.
            // We emit these as Property (kind 7) so editors can distinguish them from subs.
            NodeKind::FunctionCall { name, args } if name == "has" => {
                if let Some(first_arg) = args.first() {
                    // Extract the attribute name from a String literal (value is already
                    // unquoted per NodeKind::String doc) or an Identifier first arg.
                    let attr_name = match &first_arg.kind {
                        NodeKind::String { value, .. } => Some(value.clone()),
                        NodeKind::Identifier { name: id } => Some(id.clone()),
                        _ => None,
                    };
                    if let Some(attr) = attr_name {
                        if !attr.is_empty() {
                            let (start_line, start_char) =
                                byte_to_line_col(source, node.location.start);
                            let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                            symbols.push(LspWorkspaceSymbol {
                                name: attr,
                                kind: 7, // Property
                                location: WireLocation::new(
                                    uri.to_string(),
                                    WireRange::new(
                                        WirePosition::new(start_line, start_char),
                                        WirePosition::new(end_line, end_char),
                                    ),
                                ),
                                container_name: container
                                    .map(|s| normalize_package_separator(s).into_owned()),
                                workspace_folder_uri: None,
                            });
                        }
                    }
                }
            }

            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.extract_symbols_recursive(stmt, source, uri, container, symbols);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.extract_symbols_recursive(stmt, source, uri, container, symbols);
                }
            }

            // Recurse into expression statements so nested declarations are found
            NodeKind::ExpressionStatement { expression } => {
                self.extract_symbols_recursive(expression, source, uri, container, symbols);
            }

            _ => {
                // All other non-Declaration, non-recurse node types: no symbol emitted.
            }
        }
    }

    /// Extract simple symbols without workspace feature
    #[cfg(not(feature = "workspace"))]
    pub(crate) fn extract_simple_symbols(
        &self,
        node: &perl_parser::ast::Node,
        source: &str,
        uri: &str,
        query: &str,
        symbols: &mut Vec<serde_json::Value>,
    ) {
        use perl_parser::ast::NodeKind;

        let query_lower = query.to_lowercase();

        match &node.kind {
            NodeKind::Subroutine { name, body, .. } => {
                if let Some(sub_name) = name {
                    if sub_name.to_lowercase().contains(&query_lower) {
                        let (start_line, start_char) =
                            byte_to_line_col(source, node.location.start);
                        let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                        symbols.push(json!({
                            "name": sub_name,
                            "kind": 12, // Function
                            "location": {
                                "uri": uri,
                                "range": {
                                    "start": {"line": start_line, "character": start_char},
                                    "end": {"line": end_line, "character": end_char}
                                }
                            }
                        }));
                    }
                }
                // Recurse into body
                self.extract_simple_symbols(body, source, uri, query, symbols);
            }

            NodeKind::Package { name, block, .. } => {
                if name.to_lowercase().contains(&query_lower) {
                    let (start_line, start_char) = byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                    symbols.push(json!({
                        "name": name,
                        "kind": 2, // Module
                        "location": {
                            "uri": uri,
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char}
                            }
                        }
                    }));
                }
                // Recurse into block
                if let Some(block) = block {
                    self.extract_simple_symbols(block, source, uri, query, symbols);
                }
            }

            // Perl 5.38+ native class declaration
            NodeKind::Class { name, body, .. } => {
                if name.to_lowercase().contains(&query_lower) {
                    let (start_line, start_char) = byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                    symbols.push(json!({
                        "name": name,
                        "kind": 5, // Class
                        "location": {
                            "uri": uri,
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char}
                            }
                        }
                    }));
                }
                // Recurse into body to find methods
                self.extract_simple_symbols(body, source, uri, query, symbols);
            }

            // Perl 5.38+ native method declaration
            NodeKind::Method { name, body, .. } => {
                if name.to_lowercase().contains(&query_lower) {
                    let (start_line, start_char) = byte_to_line_col(source, node.location.start);
                    let (end_line, end_char) = byte_to_line_col(source, node.location.end);

                    symbols.push(json!({
                        "name": name,
                        "kind": 6, // Method
                        "location": {
                            "uri": uri,
                            "range": {
                                "start": {"line": start_line, "character": start_char},
                                "end": {"line": end_line, "character": end_char}
                            }
                        }
                    }));
                }
                // Recurse into body
                self.extract_simple_symbols(body, source, uri, query, symbols);
            }

            NodeKind::Program { statements } => {
                for stmt in statements {
                    self.extract_simple_symbols(stmt, source, uri, query, symbols);
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.extract_simple_symbols(stmt, source, uri, query, symbols);
                }
            }

            _ => {}
        }
    }

    /// Count references to a symbol in an AST
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn count_references(
        &self,
        node: &perl_parser::ast::Node,
        symbol_name: &str,
        symbol_kind: &str,
    ) -> usize {
        use perl_parser::ast::NodeKind;

        let mut count = 0;

        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    count += self.count_references(stmt, symbol_name, symbol_kind);
                }
            }

            NodeKind::FunctionCall { name, args } => {
                if symbol_kind == "subroutine"
                    && perl_parser::qualified_name::split_qualified_name(name).1
                        == perl_parser::qualified_name::split_qualified_name(symbol_name).1
                {
                    count += 1;
                }
                for arg in args {
                    count += self.count_references(arg, symbol_name, symbol_kind);
                }
            }

            NodeKind::MethodCall { object, method, args } => {
                if symbol_kind == "subroutine"
                    && perl_parser::qualified_name::split_qualified_name(method).1
                        == perl_parser::qualified_name::split_qualified_name(symbol_name).1
                {
                    count += 1;
                }
                count += self.count_references(object, symbol_name, symbol_kind);
                for arg in args {
                    count += self.count_references(arg, symbol_name, symbol_kind);
                }
            }

            NodeKind::Use { module, .. } => {
                if symbol_kind == "package" && module == symbol_name {
                    count += 1;
                }
            }

            NodeKind::Identifier { name } => {
                if symbol_kind == "package" && name == symbol_name {
                    count += 1;
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    count += self.count_references(stmt, symbol_name, symbol_kind);
                }
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch, .. } => {
                count += self.count_references(condition, symbol_name, symbol_kind);
                count += self.count_references(then_branch, symbol_name, symbol_kind);
                for (cond, branch) in elsif_branches {
                    count += self.count_references(cond, symbol_name, symbol_kind);
                    count += self.count_references(branch, symbol_name, symbol_kind);
                }
                if let Some(else_b) = else_branch {
                    count += self.count_references(else_b, symbol_name, symbol_kind);
                }
            }

            NodeKind::While { condition, body, continue_block, .. }
            | NodeKind::For { condition: Some(condition), body, continue_block, .. } => {
                count += self.count_references(condition, symbol_name, symbol_kind);
                count += self.count_references(body, symbol_name, symbol_kind);
                if let Some(cont) = continue_block {
                    count += self.count_references(cont, symbol_name, symbol_kind);
                }
            }

            NodeKind::Foreach { variable: _, list, body, continue_block: _ } => {
                count += self.count_references(list, symbol_name, symbol_kind);
                count += self.count_references(body, symbol_name, symbol_kind);
            }

            NodeKind::Binary { left, right, .. } => {
                count += self.count_references(left, symbol_name, symbol_kind);
                count += self.count_references(right, symbol_name, symbol_kind);
            }

            NodeKind::Unary { op, operand } => {
                // Check if this is a reference to a subroutine (\&function)
                if op == "\\" && symbol_kind == "subroutine" {
                    if let NodeKind::Identifier { name } = &operand.kind {
                        if perl_parser::qualified_name::split_qualified_name(name).1
                            == perl_parser::qualified_name::split_qualified_name(symbol_name).1
                        {
                            count += 1;
                        }
                    }
                }
                count += self.count_references(operand, symbol_name, symbol_kind);
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                count += self.count_references(condition, symbol_name, symbol_kind);
                count += self.count_references(then_expr, symbol_name, symbol_kind);
                count += self.count_references(else_expr, symbol_name, symbol_kind);
            }

            NodeKind::Assignment { lhs, rhs, op: _ } => {
                count += self.count_references(lhs, symbol_name, symbol_kind);
                count += self.count_references(rhs, symbol_name, symbol_kind);
            }

            NodeKind::Return { value } => {
                if let Some(val) = value {
                    count += self.count_references(val, symbol_name, symbol_kind);
                }
            }

            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    count += self.count_references(elem, symbol_name, symbol_kind);
                }
            }

            NodeKind::HashLiteral { pairs } => {
                for (key, val) in pairs {
                    count += self.count_references(key, symbol_name, symbol_kind);
                    count += self.count_references(val, symbol_name, symbol_kind);
                }
            }

            NodeKind::Subroutine { body, .. } => {
                count += self.count_references(body, symbol_name, symbol_kind);
            }

            NodeKind::Package { block, .. } => {
                if let Some(block) = block {
                    count += self.count_references(block, symbol_name, symbol_kind);
                }
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                count += self.count_references(body, symbol_name, symbol_kind);
                for (_var, block) in catch_blocks {
                    count += self.count_references(block, symbol_name, symbol_kind);
                }
                if let Some(finally) = finally_block {
                    count += self.count_references(finally, symbol_name, symbol_kind);
                }
            }

            // Recursively handle other node types that might contain references
            _ => {
                // Default: no references in other node types
            }
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::ast::{Node, NodeKind, SourceLocation};
    use std::io::Cursor;

    fn loc(start: usize, end: usize) -> SourceLocation {
        SourceLocation { start, end }
    }

    fn call(name: &str, start: usize) -> Node {
        Node::new(
            NodeKind::FunctionCall { name: name.to_string(), args: Vec::new() },
            loc(start, start + name.len() + 2),
        )
    }

    fn bool_node(start: usize) -> Node {
        Node::new(NodeKind::Number { value: "1".to_string() }, loc(start, start + 1))
    }

    /// Build a server instance backed by in-memory I/O (no file system needed).
    fn server() -> LspServer {
        LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()))
    }

    // ------------------------------------------------------------------
    // extract_symbols_recursive — new arms introduced by this PR
    // ------------------------------------------------------------------

    /// `our $VERSION = '1.00'` must appear as a Variable symbol (kind 13)
    /// with sigil-prefixed name `$VERSION`.
    ///
    /// This test exercises the `NodeKind::VariableDeclaration { declarator: "our" }`
    /// arm that was added to `extract_symbols_recursive` (workspace feature path).
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_our_var_emits_variable_kind() {
        // Build:  our $VERSION = '1.00';
        // Source string must be long enough so byte offsets are valid.
        let source = "our $VERSION = '1.00';\n";
        let variable_node = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "VERSION".to_string() },
            loc(4, 12),
        );
        let decl_node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "our".to_string(),
                variable: Box::new(variable_node),
                attributes: vec![],
                initializer: None,
            },
            loc(0, 22),
        );
        let root = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 23));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let ver = names.iter().position(|n| *n == "$VERSION");
        assert!(ver.is_some(), "our $VERSION should be indexed; got: {names:?}");
        assert_eq!(
            symbols.get(ver.unwrap_or(0)).map(|s| s.kind),
            Some(13),
            "$VERSION should have LSP kind 13 (Variable)"
        );
    }

    /// `my $local` must NOT appear in the symbol list — only `our` is indexed.
    ///
    /// Negative test for the `declarator == "our"` guard in the
    /// `NodeKind::VariableDeclaration` arm.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_my_var_not_indexed() {
        let source = "my $local = 1;\n";
        let variable_node = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "local".to_string() },
            loc(3, 9),
        );
        let decl_node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(variable_node),
                attributes: vec![],
                initializer: None,
            },
            loc(0, 14),
        );
        let root = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 15));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"$local"), "my $local must NOT be indexed; got: {names:?}");
    }

    /// `has 'name' => (...)` must appear as a Property symbol (kind 7) named
    /// `name` (no sigil).
    ///
    /// This test exercises the `NodeKind::FunctionCall { name: "has" }` arm
    /// that was added to `extract_symbols_recursive`.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_has_attr_emits_property_kind() {
        // Build:  has 'name' => (is => 'ro');
        let source = "has 'name' => (is => 'ro');\n";
        let attr_name_node = Node::new(
            NodeKind::String { value: "name".to_string(), interpolated: false },
            loc(4, 10),
        );
        let has_call = Node::new(
            NodeKind::FunctionCall { name: "has".to_string(), args: vec![attr_name_node] },
            loc(0, 27),
        );
        let root = Node::new(NodeKind::Program { statements: vec![has_call] }, loc(0, 28));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let pos = names.iter().position(|n| *n == "name");
        assert!(pos.is_some(), "has 'name' should be indexed as a Property; got: {names:?}");
        assert_eq!(
            symbols.get(pos.unwrap_or(0)).map(|s| s.kind),
            Some(7),
            "'name' attribute should have LSP kind 7 (Property)"
        );
    }

    /// `has name => (...)` where the first arg is a bare `Identifier` (not a quoted
    /// string) must still emit a Property symbol (kind 7).
    ///
    /// Covers the `NodeKind::Identifier { name: id }` branch of the inner match.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_has_identifier_arg_emits_property_kind() {
        let source = "has name => (is => 'ro');\n";
        let attr_name_node =
            Node::new(NodeKind::Identifier { name: "name".to_string() }, loc(4, 8));
        let has_call = Node::new(
            NodeKind::FunctionCall { name: "has".to_string(), args: vec![attr_name_node] },
            loc(0, 25),
        );
        let root = Node::new(NodeKind::Program { statements: vec![has_call] }, loc(0, 26));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let pos = names.iter().position(|n| *n == "name");
        assert!(
            pos.is_some(),
            "has name (Identifier arg) should be indexed as Property; got: {names:?}"
        );
        assert_eq!(
            symbols.get(pos.unwrap_or(0)).map(|s| s.kind),
            Some(7),
            "Identifier-arg attribute should have kind 7 (Property)"
        );
    }

    /// `our $VERSION` inside a `{ ... }` block must still be indexed.
    ///
    /// Covers the `NodeKind::Block { statements }` recursion arm added to
    /// `extract_symbols_recursive`.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_our_var_inside_block_is_indexed() {
        let source = "{ our $VERSION = '1.00'; }\n";
        let variable_node = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "VERSION".to_string() },
            loc(6, 14),
        );
        let decl_node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "our".to_string(),
                variable: Box::new(variable_node),
                attributes: vec![],
                initializer: None,
            },
            loc(2, 24),
        );
        let block = Node::new(NodeKind::Block { statements: vec![decl_node] }, loc(0, 26));
        let root = Node::new(NodeKind::Program { statements: vec![block] }, loc(0, 27));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"$VERSION"),
            "our $VERSION inside a block should be indexed; got: {names:?}"
        );
    }

    /// `our $VERSION` wrapped in an `ExpressionStatement` must still be indexed.
    ///
    /// Covers the `NodeKind::ExpressionStatement { expression }` recursion arm
    /// added to `extract_symbols_recursive`.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_our_var_inside_expression_statement_is_indexed() {
        let source = "our $EPOCH = time();\n";
        let variable_node = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "EPOCH".to_string() },
            loc(4, 10),
        );
        let decl_node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "our".to_string(),
                variable: Box::new(variable_node),
                attributes: vec![],
                initializer: None,
            },
            loc(0, 20),
        );
        let expr_stmt = Node::new(
            NodeKind::ExpressionStatement { expression: Box::new(decl_node) },
            loc(0, 21),
        );
        let root = Node::new(NodeKind::Program { statements: vec![expr_stmt] }, loc(0, 22));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"$EPOCH"),
            "our $EPOCH inside ExpressionStatement should be indexed; got: {names:?}"
        );
    }

    /// `has` with an unrecognised first-arg kind must NOT produce a symbol
    /// (exercises the `_ => None` wildcard branch in the inner match).
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_has_unknown_first_arg_produces_no_symbol() {
        let source = "has $attr_ref => (is => 'ro');\n";
        // Use a Variable node as the first arg — not String or Identifier.
        let var_arg = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "attr_ref".to_string() },
            loc(4, 13),
        );
        let has_call = Node::new(
            NodeKind::FunctionCall { name: "has".to_string(), args: vec![var_arg] },
            loc(0, 30),
        );
        let root = Node::new(NodeKind::Program { statements: vec![has_call] }, loc(0, 31));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.is_empty(),
            "has with Variable first arg must not produce a symbol; got: {names:?}"
        );
    }

    // ------------------------------------------------------------------
    // NodeKindCategory drift-guard tests (PR #1330)
    // ------------------------------------------------------------------
    // These tests verify that the NodeKindCategory::Declaration guard in
    // extract_symbols_recursive correctly filters declaration types and
    // preserves all 6 symbol-kind mappings: Subroutine→Function(12),
    // Package→Module(2), Class→Class(5), Method→Method(6),
    // VariableDeclaration{our}→Variable(13), FunctionCall{has}→Property(7).
    //
    // MECHANISM (drift-guard — centralization enforced by convention, compile-time
    // enforcement lives in perl_ast::classification):
    // - The outer guard `kind if kind.category() == Declaration => { ... }` funnels
    //   all Declaration-category variants into one match arm, centralizing policy.
    // - The real compile-time enforcement is in `perl_ast::classification::category()`:
    //   that match has NO wildcard arm, so adding a new NodeKind variant is a compile
    //   error in classification.rs until category() and flags() are both extended.
    // - Once classification.rs is updated, the new Declaration variant reaches the
    //   inner match here. The inner match has a `_ => {}` wildcard that silently
    //   ignores un-indexed Declaration variants (Use, No, PhaseBlock, etc.).
    // - Convention: when adding a new Declaration NodeKind, the developer must
    //   explicitly decide here: index it (add an arm) or leave silent (`_` covers it).
    // - FunctionCall{name=="has"} is Expression-category (NOT Declaration) and is
    //   handled as a separate outer arm below, outside this guard.
    //
    // RUNTIME TESTS below verify that the 6 declaration-emitting cases still
    // produce their correct LSP symbol kinds after the refactoring.

    /// `sub test_sub { }` must emit kind 12 (Function).
    /// Characterization test for Subroutine → Function mapping.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_subroutine_emits_function() {
        let source = "sub test_sub { }\n";
        let sub_node = Node::new(
            NodeKind::Subroutine {
                name: Some("test_sub".to_string()),
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(10, 13))),
            },
            loc(0, 15),
        );
        let root = Node::new(NodeKind::Program { statements: vec![sub_node] }, loc(0, 16));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let idx = names.iter().position(|n| *n == "test_sub");
        assert!(idx.is_some(), "Subroutine 'test_sub' should produce a symbol; got: {names:?}");
        assert_eq!(
            symbols.get(idx.unwrap()).map(|s| s.kind),
            Some(12),
            "Subroutine should have LSP kind 12 (Function)"
        );
    }

    /// `package Foo;` must emit kind 2 (Module).
    /// Characterization test for Package → Module mapping.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_package_emits_module() {
        let source = "package Foo;\n";
        let pkg_node = Node::new(
            NodeKind::Package {
                name: "Foo".to_string(),
                name_span: loc(8, 11),
                block: Some(Box::new(Node::new(
                    NodeKind::Block { statements: vec![] },
                    loc(10, 11),
                ))),
            },
            loc(0, 12),
        );
        let root = Node::new(NodeKind::Program { statements: vec![pkg_node] }, loc(0, 13));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let idx = names.iter().position(|n| *n == "Foo");
        assert!(idx.is_some(), "Package 'Foo' should produce a symbol; got: {names:?}");
        assert_eq!(
            symbols.get(idx.unwrap()).map(|s| s.kind),
            Some(2),
            "Package should have LSP kind 2 (Module)"
        );
    }

    /// `class MyClass { }` must emit kind 5 (Class).
    /// Characterization test for Class → Class mapping.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_class_emits_class() {
        let source = "class MyClass { }\n";
        let class_node = Node::new(
            NodeKind::Class {
                name: "MyClass".to_string(),
                parents: vec![],
                body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(12, 14))),
            },
            loc(0, 17),
        );
        let root = Node::new(NodeKind::Program { statements: vec![class_node] }, loc(0, 18));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let idx = names.iter().position(|n| *n == "MyClass");
        assert!(idx.is_some(), "Class 'MyClass' should produce a symbol; got: {names:?}");
        assert_eq!(
            symbols.get(idx.unwrap()).map(|s| s.kind),
            Some(5),
            "Class should have LSP kind 5 (Class)"
        );
    }

    /// `method my_method { }` must emit kind 6 (Method).
    /// Characterization test for Method → Method mapping.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_method_emits_method() {
        let source = "method my_method { }\n";
        let method_node = Node::new(
            NodeKind::Method {
                name: "my_method".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, loc(15, 17))),
            },
            loc(0, 20),
        );
        let root = Node::new(NodeKind::Program { statements: vec![method_node] }, loc(0, 21));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let idx = names.iter().position(|n| *n == "my_method");
        assert!(idx.is_some(), "Method 'my_method' should produce a symbol; got: {names:?}");
        assert_eq!(
            symbols.get(idx.unwrap()).map(|s| s.kind),
            Some(6),
            "Method should have LSP kind 6 (Method)"
        );
    }

    /// `our $VERSION;` must emit kind 13 (Variable) with sigil-prefixed name.
    /// Characterization test for VariableDeclaration{our} → Variable mapping.
    /// (Complements the existing extract_symbols_our_var_emits_variable_kind test
    /// as part of the drift-guard characterization suite.)
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_our_var_characterization() {
        let source = "our $VERSION = '1.0';\n";
        let variable_node = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "VERSION".to_string() },
            loc(4, 12),
        );
        let decl_node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "our".to_string(),
                variable: Box::new(variable_node),
                attributes: vec![],
                initializer: None,
            },
            loc(0, 22),
        );
        let root = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 23));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let idx = names.iter().position(|n| *n == "$VERSION");
        assert!(idx.is_some(), "our $VERSION should produce a symbol; got: {names:?}");
        assert_eq!(
            symbols.get(idx.unwrap()).map(|s| s.kind),
            Some(13),
            "our variable should have LSP kind 13 (Variable)"
        );
    }

    /// `has 'attr_name' => (...)` must emit kind 7 (Property).
    /// Characterization test for FunctionCall{name=="has"} → Property mapping.
    /// (Complements the existing extract_symbols_has_attr_emits_property_kind test
    /// as part of the drift-guard characterization suite.)
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_has_attr_characterization() {
        let source = "has 'attr_name' => (is => 'ro');\n";
        let attr_node = Node::new(
            NodeKind::String { value: "attr_name".to_string(), interpolated: false },
            loc(4, 15),
        );
        let has_call = Node::new(
            NodeKind::FunctionCall { name: "has".to_string(), args: vec![attr_node] },
            loc(0, 32),
        );
        let root = Node::new(NodeKind::Program { statements: vec![has_call] }, loc(0, 33));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let idx = names.iter().position(|n| *n == "attr_name");
        assert!(idx.is_some(), "has 'attr_name' should produce a Property symbol; got: {names:?}");
        assert_eq!(
            symbols.get(idx.unwrap()).map(|s| s.kind),
            Some(7),
            "has attribute should have LSP kind 7 (Property)"
        );
    }

    /// Non-Declaration nodes (my variables, Use, No, etc.) must NOT emit symbols
    /// when passed through the declaration guard.
    /// This test documents that the guard correctly filters non-declaration types.
    #[cfg(feature = "workspace")]
    #[test]
    fn extract_symbols_my_var_filtered_by_declaration_guard() {
        // Verify that `my $local;` (which is VariableDeclaration{declarator: "my"})
        // does NOT produce a symbol. This is an edge case where a Declaration node
        // exists but the internal guard `declarator == "our"` prevents emission.
        let source = "my $local = 1;\n";
        let variable_node = Node::new(
            NodeKind::Variable { sigil: "$".to_string(), name: "local".to_string() },
            loc(3, 9),
        );
        let decl_node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(variable_node),
                attributes: vec![],
                initializer: None,
            },
            loc(0, 14),
        );
        let root = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 15));

        let symbols = server().extract_document_symbols(&root, source, "file:///test.pl");

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            !names.contains(&"$local"),
            "my $local must NOT be indexed (handled by inner declarator guard)"
        );
    }

    // ------------------------------------------------------------------

    #[test]
    fn count_references_visits_if_and_while_children_with_keyword_metadata() {
        let server =
            LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(Vec::<u8>::new()));
        let if_node = Node::new(
            NodeKind::If {
                condition: Box::new(call("target", 1)),
                then_branch: Box::new(call("target", 10)),
                elsif_branches: vec![(Box::new(bool_node(20)), Box::new(call("target", 24)))],
                else_branch: Some(Box::new(call("target", 34))),
                keyword: Some("unless".to_string()),
            },
            loc(0, 42),
        );
        let while_node = Node::new(
            NodeKind::While {
                condition: Box::new(call("target", 44)),
                body: Box::new(call("target", 54)),
                continue_block: Some(Box::new(call("target", 64))),
                keyword: Some("until".to_string()),
            },
            loc(43, 72),
        );
        let root =
            Node::new(NodeKind::Program { statements: vec![if_node, while_node] }, loc(0, 72));

        let count = server.count_references(&root, "target", "subroutine");

        assert_eq!(count, 7);
    }
}
