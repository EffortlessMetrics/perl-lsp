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
        use perl_parser::ast::NodeKind;

        match &node.kind {
            NodeKind::Subroutine { name, body, .. } => {
                // Add the subroutine as a symbol if it has a name
                if let Some(sub_name) = name {
                    let (start_line, start_char) = byte_to_line_col(source, node.location.start);
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
                let (start_line, start_char) = byte_to_line_col(source, node.location.start);
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
                    container_name: container.map(|s| normalize_package_separator(s).into_owned()),
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
                let (start_line, start_char) = byte_to_line_col(source, node.location.start);
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
                    container_name: container.map(|s| normalize_package_separator(s).into_owned()),
                    workspace_folder_uri: None,
                });

                // Recurse into body with this class as container
                self.extract_symbols_recursive(body, source, uri, Some(name.as_str()), symbols);
            }

            // Perl 5.38+ native method declaration
            NodeKind::Method { name, body, .. } => {
                let (start_line, start_char) = byte_to_line_col(source, node.location.start);
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
                    container_name: container.map(|s| normalize_package_separator(s).into_owned()),
                    workspace_folder_uri: None,
                });

                // Recurse into body with this method as container
                self.extract_symbols_recursive(body, source, uri, Some(name.as_str()), symbols);
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

            _ => {
                // For other node types, recurse into children if they might contain symbols
                // This is a simplified version - you might want to handle more node types
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
                if symbol_kind == "subroutine" && perl_parser::qualified_name::split_qualified_name(name).1 == perl_parser::qualified_name::split_qualified_name(symbol_name).1 {
                    count += 1;
                }
                for arg in args {
                    count += self.count_references(arg, symbol_name, symbol_kind);
                }
            }

            NodeKind::MethodCall { object, method, args } => {
                if symbol_kind == "subroutine" && perl_parser::qualified_name::split_qualified_name(method).1 == perl_parser::qualified_name::split_qualified_name(symbol_name).1 {
                    count += 1;
                }
                count += self.count_references(object, symbol_name, symbol_kind);
                for arg in args {
                    count += self.count_references(arg, symbol_name, symbol_kind);
                }
            }

            NodeKind::Use { module, .. } => {
                if symbol_kind == "package" && perl_parser::qualified_name::split_qualified_name(module).1 == perl_parser::qualified_name::split_qualified_name(symbol_name).1 {
                    count += 1;
                }
            }

            NodeKind::Identifier { name } => {
                if symbol_kind == "package" && perl_parser::qualified_name::split_qualified_name(name).1 == perl_parser::qualified_name::split_qualified_name(symbol_name).1 {
                    count += 1;
                }
            }

            NodeKind::Block { statements } => {
                for stmt in statements {
                    count += self.count_references(stmt, symbol_name, symbol_kind);
                }
            }

            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
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

            NodeKind::While { condition, body, continue_block }
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
                        if perl_parser::qualified_name::split_qualified_name(name).1 == perl_parser::qualified_name::split_qualified_name(symbol_name).1 {
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
