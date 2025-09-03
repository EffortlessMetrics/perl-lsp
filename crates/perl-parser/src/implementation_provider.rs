//! Implementation provider for finding implementations of types/interfaces
//!
//! This provider finds:
//! - Subclasses that inherit from a base class
//! - Overridden methods in derived classes

use crate::ast::{Node, NodeKind};
use crate::uri::parse_uri;
use crate::workspace_index::WorkspaceIndex;
use lsp_types::{LocationLink, Position, Range};
use std::collections::HashMap;

pub struct ImplementationProvider {
    workspace_index: Option<std::sync::Arc<WorkspaceIndex>>,
}

impl ImplementationProvider {
    pub fn new(workspace_index: Option<std::sync::Arc<WorkspaceIndex>>) -> Self {
        Self { workspace_index }
    }

    /// Find implementations at the given position
    pub fn find_implementations(
        &self,
        ast: &Node,
        line: u32,
        character: u32,
        uri: &str,
        documents: &HashMap<String, String>,
    ) -> Vec<LocationLink> {
        // Find the node at position
        let target_node = match self.find_node_at_position(ast, line, character, documents.get(uri))
        {
            Some(node) => node,
            None => return Vec::new(),
        };

        // Extract what we're looking for implementations of
        match self.extract_implementation_target(&target_node) {
            Some(ImplementationTarget::Package(name)) => {
                self.find_package_implementations(&name, documents)
            }
            Some(ImplementationTarget::Method { package, method }) => {
                self.find_method_implementations(&package, &method, documents)
            }
            None => Vec::new(),
        }
    }

    /// Find all implementations of a package (subclasses)
    fn find_package_implementations(
        &self,
        base_package: &str,
        documents: &HashMap<String, String>,
    ) -> Vec<LocationLink> {
        let mut results = Vec::new();

        // Build inheritance index from all documents
        let hierarchy_provider = TypeHierarchyProvider::new();

        for (uri, content) in documents {
            // Parse document
            if let Ok(ast) = crate::Parser::new(content).parse() {
                // Find packages that inherit from base_package
                self.find_inheriting_packages(&ast, base_package, uri, &mut results);
            }
        }

        // If we have workspace index, use it for more comprehensive results
        if let Some(ref index) = self.workspace_index {
            // Query workspace index for implementations
            let symbols = index.find_symbols(base_package);
            for symbol in symbols {
                if symbol.kind == crate::workspace_index::SymbolKind::Class
                    || symbol.kind == crate::workspace_index::SymbolKind::Package
                {
                    // Check if this symbol inherits from base_package
                    if let Some(container) = &symbol.container_name {
                        if container.contains(base_package) {
                            let target_uri = parse_uri(&symbol.uri);
                            results.push(LocationLink {
                                origin_selection_range: None,
                                target_uri,
                                target_range: symbol.range,
                                target_selection_range: symbol.range,
                            });
                        }
                    }
                }
            }
        }

        results
    }

    /// Find method implementations (overrides) in subclasses
    fn find_method_implementations(
        &self,
        package: &str,
        method: &str,
        documents: &HashMap<String, String>,
    ) -> Vec<LocationLink> {
        let mut results = Vec::new();

        // First find all subclasses
        let subclasses = self.find_package_implementations(package, documents);

        // Then find the method in each subclass
        for subclass_link in &subclasses {
            if let Some(doc_content) = documents.get(subclass_link.target_uri.as_str()) {
                if let Ok(ast) = crate::Parser::new(doc_content).parse() {
                    self.find_method_in_ast(
                        &ast,
                        method,
                        subclass_link.target_uri.as_str(),
                        &mut results,
                    );
                }
            }
        }

        results
    }

    /// Find packages that inherit from base_package in AST
    fn find_inheriting_packages(
        &self,
        node: &Node,
        base_package: &str,
        uri: &str,
        results: &mut Vec<LocationLink>,
    ) {
        let mut current_package = String::new();
        self.find_inheriting_packages_recursive(
            node,
            base_package,
            uri,
            &mut current_package,
            results,
        );
    }

    fn find_inheriting_packages_recursive(
        &self,
        node: &Node,
        base_package: &str,
        uri: &str,
        current_package: &mut String,
        results: &mut Vec<LocationLink>,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } => {
                *current_package = name.clone();
            }
            NodeKind::Use { module, args, .. } if module == "base" || module == "parent" => {
                // Check if any arg matches base_package
                for arg in args {
                    if arg == base_package {
                        // Current package inherits from base_package
                        let target_uri = parse_uri(uri);
                        results.push(LocationLink {
                            origin_selection_range: None,
                            target_uri,
                            target_range: self.node_to_range(node),
                            target_selection_range: self.node_to_range(node),
                        });
                    }
                }
            }
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                if declarator == "our" {
                    if let NodeKind::Variable { sigil, name } = &variable.kind {
                        if sigil == "@" && name == "ISA" {
                            if let Some(init) = initializer {
                                if self.contains_parent(init, base_package) {
                                    let target_uri = parse_uri(uri);
                                    results.push(LocationLink {
                                        origin_selection_range: None,
                                        target_uri,
                                        target_range: self.node_to_range(node),
                                        target_selection_range: self.node_to_range(node),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_inheriting_packages_recursive(
                        stmt,
                        base_package,
                        uri,
                        current_package,
                        results,
                    );
                }
            }
            _ => {}
        }
    }

    /// Find method definitions in AST
    fn find_method_in_ast(
        &self,
        node: &Node,
        method_name: &str,
        uri: &str,
        results: &mut Vec<LocationLink>,
    ) {
        match &node.kind {
            NodeKind::Subroutine { name: Some(name), .. } if name == method_name => {
                let target_uri = parse_uri(uri);
                results.push(LocationLink {
                    origin_selection_range: None,
                    target_uri,
                    target_range: self.node_to_range(node),
                    target_selection_range: self.node_to_range(node),
                });
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_method_in_ast(stmt, method_name, uri, results);
                }
            }
            _ => {}
        }
    }

    /// Extract implementation target from node
    fn extract_implementation_target(&self, node: &Node) -> Option<ImplementationTarget> {
        match &node.kind {
            NodeKind::Package { name, .. } => Some(ImplementationTarget::Package(name.clone())),
            NodeKind::Subroutine { name: Some(method), .. } => {
                // Would need to track enclosing package
                Some(ImplementationTarget::Method {
                    package: "main".to_string(),
                    method: method.clone(),
                })
            }
            NodeKind::Identifier { name } if name.contains("::") => {
                let parts: Vec<&str> = name.split("::").collect();
                if parts.len() == 2 {
                    Some(ImplementationTarget::Method {
                        package: parts[0].to_string(),
                        method: parts[1].to_string(),
                    })
                } else if parts.len() > 2 {
                    Some(ImplementationTarget::Package(parts[..parts.len() - 1].join("::")))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Find node at position
    fn find_node_at_position(
        &self,
        node: &Node,
        line: u32,
        character: u32,
        source: Option<&String>,
    ) -> Option<Node> {
        if let Some(src) = source {
            let (start_line, start_col) =
                crate::position::offset_to_utf16_line_col(src, node.location.start);
            let (end_line, end_col) =
                crate::position::offset_to_utf16_line_col(src, node.location.end);

            if line >= start_line && line <= end_line {
                if (line == start_line && character >= start_col)
                    || (line == end_line && character <= end_col)
                    || (line > start_line && line < end_line)
                {
                    // Check children first for more specific match
                    match &node.kind {
                        NodeKind::Program { statements } | NodeKind::Block { statements } => {
                            for stmt in statements {
                                if let Some(child) =
                                    self.find_node_at_position(stmt, line, character, source)
                                {
                                    return Some(child);
                                }
                            }
                        }
                        _ => {}
                    }
                    return Some(node.clone());
                }
            }
        }
        None
    }

    /// Convert node to LSP range
    fn node_to_range(&self, _node: &Node) -> Range {
        Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 0 } }
    }

    /// Extract parent name from use statement argument (not needed anymore)
    fn _extract_parent_name(&self, node: &Node) -> Option<String> {
        match &node.kind {
            NodeKind::String { value, .. } => Some(value.clone()),
            NodeKind::Identifier { name } => Some(name.clone()),
            _ => None,
        }
    }

    /// Check if initializer contains parent
    fn contains_parent(&self, node: &Node, parent: &str) -> bool {
        match &node.kind {
            NodeKind::String { value, .. } => value == parent,
            NodeKind::ArrayLiteral { elements, .. } => {
                elements.iter().any(|e| self.contains_parent(e, parent))
            }
            _ => false,
        }
    }
}

#[allow(dead_code)]
enum ImplementationTarget {
    Package(String),
    Method { package: String, method: String },
    BlessedType(String),
}
