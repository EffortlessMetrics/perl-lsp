//! Implementation provider for finding implementations of types/interfaces
//!
//! This provider finds:
//! - Subclasses that inherit from a base class
//! - Overridden methods in derived classes

use crate::type_hierarchy::TypeHierarchyProvider;
use crate::util::uri::parse_uri;
use lsp_types::LocationLink;
use lsp_types::{Position as LspPosition, Range as LspRange};
use perl_parser::ast::{Node, NodeKind};
use perl_parser::workspace_index::WorkspaceIndex;
use std::collections::HashMap;

/// Provider for finding implementations of types and interfaces in Perl code
///
/// This provider locates implementations and inheritance relationships in Perl codebases:
/// - Subclasses that inherit from a base class using `@ISA` or `use parent`
/// - Overridden methods in derived classes
/// - Package implementations and blessed type relationships
///
/// # LSP Workflow Integration
///
/// Integrates with the Parse → Index → Navigate → Complete → Analyze workflow:
/// - **Parse**: AST analysis identifies package and method definitions
/// - **Index**: Workspace indexing tracks inheritance relationships
/// - **Navigate**: Provides go-to-implementation functionality
/// - **Complete**: No direct integration (implementations don't affect completion)
/// - **Analyze**: Implementation analysis supports refactoring decisions
///
/// # Performance
///
/// - Implementation finding: <100ms for typical inheritance hierarchies
/// - Memory usage: <5MB for implementation metadata
/// - Workspace indexing: Leverages cached inheritance relationships
pub struct ImplementationProvider {
    workspace_index: Option<std::sync::Arc<WorkspaceIndex>>,
}

impl ImplementationProvider {
    /// Create a new implementation provider with optional workspace indexing
    ///
    /// # Arguments
    ///
    /// * `workspace_index` - Optional workspace index for cross-file inheritance tracking
    ///
    /// # Returns
    ///
    /// A new [`ImplementationProvider`] configured for finding Perl implementations
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_lsp_navigation::ImplementationProvider;
    ///
    /// // Without workspace indexing (single-file analysis)
    /// let provider = ImplementationProvider::new(None);
    ///
    /// // With workspace indexing (cross-file inheritance)
    /// # use std::sync::Arc;
    /// # use perl_parser::workspace_index::WorkspaceIndex;
    /// # let workspace_index = Arc::new(WorkspaceIndex::new());
    /// let provider = ImplementationProvider::new(Some(workspace_index));
    /// ```
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
        let source = documents.get(uri);
        let target_node = match self.find_node_at_position(ast, line, character, source) {
            Some(node) => node,
            None => return Vec::new(),
        };

        // Compute the enclosing package for the cursor position (Fix B).
        // This is needed when the cursor lands on a Subroutine node so that
        // `extract_implementation_target` can use the real package instead of "main".
        let byte_offset = source
            .map(|src| crate::position::utf16_line_col_to_offset(src, line, character))
            .unwrap_or(0);
        let current_package = crate::declaration::current_package_at(ast, byte_offset);

        // Extract what we're looking for implementations of
        match self.extract_implementation_target(&target_node, current_package) {
            Some(ImplementationTarget::Package(name)) => {
                self.find_package_implementations(&name, documents)
            }
            Some(ImplementationTarget::Method { package, method }) => {
                self.find_method_implementations(&package, &method, documents)
            }
            Some(ImplementationTarget::BlessedType(name)) => {
                // For blessed types, find package implementations
                self.find_package_implementations(&name, documents)
            }
            None => Vec::new(),
        }
    }

    /// Find all implementations of a package (subclasses).
    fn find_package_implementations(
        &self,
        base_package: &str,
        documents: &HashMap<String, String>,
    ) -> Vec<LocationLink> {
        self.find_subclass_locations(base_package, documents)
            .into_iter()
            .map(|(_name, link)| link)
            .collect()
    }

    /// Find subclasses with their package names, for use in method resolution.
    ///
    /// Returns `(subclass_package_name, LocationLink_pointing_to_package_declaration)`.
    fn find_subclass_locations(
        &self,
        base_package: &str,
        documents: &HashMap<String, String>,
    ) -> Vec<(String, LocationLink)> {
        let mut results: Vec<(String, LocationLink)> = Vec::new();

        let _hierarchy_provider = TypeHierarchyProvider::new();

        for (uri, content) in documents {
            if let Ok(ast) = crate::Parser::new(content).parse() {
                self.find_inheriting_packages_named(&ast, base_package, uri, content, &mut results);
            }
        }

        // If we have workspace index, use it for more comprehensive results
        if let Some(ref index) = self.workspace_index {
            let symbols = index.find_symbols(base_package);
            for symbol in symbols {
                if symbol.kind == crate::workspace_index::SymbolKind::Class
                    || symbol.kind == crate::workspace_index::SymbolKind::Package
                {
                    if let Some(container) = &symbol.container_name {
                        if container.contains(base_package) {
                            let target_uri = parse_uri(&symbol.uri);
                            // Convert internal Position to LSP Position (LSP uses 0-based, internal uses 1-based)
                            let lsp_start = LspPosition::new(
                                symbol.range.start.line - 1,
                                symbol.range.start.column - 1,
                            );
                            let lsp_end = LspPosition::new(
                                symbol.range.end.line - 1,
                                symbol.range.end.column - 1,
                            );
                            results.push((
                                symbol.name.clone(),
                                LocationLink {
                                    origin_selection_range: None,
                                    target_uri,
                                    target_range: LspRange::new(lsp_start, lsp_end),
                                    target_selection_range: LspRange::new(lsp_start, lsp_end),
                                },
                            ));
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

        // First find all subclasses (with their package names for scoped method lookup)
        let subclasses = self.find_subclass_locations(package, documents);

        // Then find the method in each subclass, restricted to the subclass package scope
        for (subclass_name, subclass_link) in &subclasses {
            if let Some(doc_content) = documents.get(subclass_link.target_uri.as_str()) {
                if let Ok(ast) = crate::Parser::new(doc_content).parse() {
                    self.find_method_in_package(
                        &ast,
                        method,
                        subclass_name,
                        subclass_link.target_uri.as_str(),
                        doc_content,
                        &mut results,
                    );
                }
            }
        }

        results
    }

    /// Find packages that inherit from base_package in AST, returning named pairs.
    fn find_inheriting_packages_named(
        &self,
        node: &Node,
        base_package: &str,
        uri: &str,
        source: &str,
        results: &mut Vec<(String, LocationLink)>,
    ) {
        let mut current_package = String::new();
        // Track the package node's range so results point to `package Foo;` not `use parent` (Fix C).
        let mut current_package_range = LspRange::default();
        self.find_inheriting_packages_recursive(
            node,
            base_package,
            uri,
            source,
            &mut current_package,
            &mut current_package_range,
            results,
        );
    }

    fn find_inheriting_packages_recursive(
        &self,
        node: &Node,
        base_package: &str,
        uri: &str,
        source: &str,
        current_package: &mut String,
        current_package_range: &mut LspRange,
        results: &mut Vec<(String, LocationLink)>,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } => {
                *current_package = name.clone();
                // Record this package node's range for use when we later find `use parent` (Fix C).
                *current_package_range = self.node_to_range(node, source);
            }
            NodeKind::Use { module, args, .. } if module == "base" || module == "parent" => {
                // Check if any arg matches base_package.
                // Args are raw token texts (e.g., `'Animal'` with quotes), so strip them first.
                for arg in args {
                    let normalized = Self::strip_arg_quotes(arg);
                    if normalized == base_package {
                        // Point at the enclosing package declaration, not the `use parent` line (Fix C).
                        let pkg_range = *current_package_range;
                        let target_uri = parse_uri(uri);
                        results.push((
                            current_package.clone(),
                            LocationLink {
                                origin_selection_range: None,
                                target_uri,
                                target_range: pkg_range,
                                target_selection_range: pkg_range,
                            },
                        ));
                    }
                }
            }
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                if declarator == "our" {
                    if let NodeKind::Variable { sigil, name } = &variable.kind {
                        if sigil == "@" && name == "ISA" {
                            if let Some(init) = initializer {
                                if self.contains_parent(init, base_package) {
                                    let pkg_range = *current_package_range;
                                    let target_uri = parse_uri(uri);
                                    results.push((
                                        current_package.clone(),
                                        LocationLink {
                                            origin_selection_range: None,
                                            target_uri,
                                            target_range: pkg_range,
                                            target_selection_range: pkg_range,
                                        },
                                    ));
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
                        source,
                        current_package,
                        current_package_range,
                        results,
                    );
                }
            }
            _ => {}
        }
    }

    /// Find method definitions in AST (any package). Kept for future workspace-index integration.
    #[allow(dead_code)]
    fn find_method_in_ast(
        &self,
        node: &Node,
        method_name: &str,
        uri: &str,
        source: &str,
        results: &mut Vec<LocationLink>,
    ) {
        match &node.kind {
            NodeKind::Subroutine { name: Some(name), .. } if name == method_name => {
                let target_uri = parse_uri(uri);
                results.push(LocationLink {
                    origin_selection_range: None,
                    target_uri,
                    target_range: self.node_to_range(node, source),
                    target_selection_range: self.node_to_range(node, source),
                });
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.find_method_in_ast(stmt, method_name, uri, source, results);
                }
            }
            _ => {}
        }
    }

    /// Find a method defined within a specific package in the AST.
    ///
    /// Only emits a result when `method_name` appears after `package_name;` in
    /// linear-form source (before the next package declaration), or inside a
    /// block-form package (`package Name { ... }`). This prevents cross-package
    /// false positives when all subclasses live in the same file.
    fn find_method_in_package(
        &self,
        node: &Node,
        method_name: &str,
        package_name: &str,
        uri: &str,
        source: &str,
        results: &mut Vec<LocationLink>,
    ) {
        let mut current_package: Option<String> = None;
        self.find_method_in_package_with_scope(
            node,
            method_name,
            package_name,
            uri,
            source,
            &mut current_package,
            results,
        );
    }

    fn find_method_in_package_with_scope(
        &self,
        node: &Node,
        method_name: &str,
        package_name: &str,
        uri: &str,
        source: &str,
        current_package: &mut Option<String>,
        results: &mut Vec<LocationLink>,
    ) {
        if let NodeKind::Program { statements } | NodeKind::Block { statements } = &node.kind {
            for stmt in statements {
                match &stmt.kind {
                    NodeKind::Package { name, block: Some(inner), .. } => {
                        let previous_package = current_package.clone();
                        *current_package = Some(name.clone());
                        self.find_method_in_package_with_scope(
                            inner,
                            method_name,
                            package_name,
                            uri,
                            source,
                            current_package,
                            results,
                        );
                        *current_package = previous_package;
                    }
                    NodeKind::Package { name, .. } => {
                        *current_package = Some(name.clone());
                    }
                    NodeKind::Subroutine { name: Some(sub_name), .. } => {
                        if current_package.as_deref() == Some(package_name)
                            && *sub_name == method_name
                        {
                            let target_uri = parse_uri(uri);
                            results.push(LocationLink {
                                origin_selection_range: None,
                                target_uri,
                                target_range: self.node_to_range(stmt, source),
                                target_selection_range: self.node_to_range(stmt, source),
                            });
                        }
                    }
                    _ => {
                        self.find_method_in_package_with_scope(
                            stmt,
                            method_name,
                            package_name,
                            uri,
                            source,
                            current_package,
                            results,
                        );
                    }
                }
            }
            return;
        }

        if let NodeKind::Package { name, block: Some(inner), .. } = &node.kind {
            let previous_package = current_package.clone();
            *current_package = Some(name.clone());
            self.find_method_in_package_with_scope(
                inner,
                method_name,
                package_name,
                uri,
                source,
                current_package,
                results,
            );
            *current_package = previous_package;
        }
    }

    /// Extract implementation target from node.
    ///
    /// `current_package` is the package name enclosing the cursor position,
    /// determined by `crate::declaration::current_package_at`. It replaces the
    /// previous hardcoded `"main"` for `Subroutine` nodes (Fix B).
    fn extract_implementation_target(
        &self,
        node: &Node,
        current_package: &str,
    ) -> Option<ImplementationTarget> {
        match &node.kind {
            NodeKind::Package { name, .. } => Some(ImplementationTarget::Package(name.clone())),
            NodeKind::Subroutine { name: Some(method), .. } => Some(ImplementationTarget::Method {
                package: current_package.to_string(),
                method: method.clone(),
            }),
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
    fn node_to_range(&self, node: &Node, source: &str) -> LspRange {
        let (start_line, start_col) =
            crate::position::offset_to_utf16_line_col(source, node.location.start);
        let (end_line, end_col) =
            crate::position::offset_to_utf16_line_col(source, node.location.end);

        LspRange {
            start: LspPosition::new(start_line, start_col),
            end: LspPosition::new(end_line, end_col),
        }
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

    /// Strip surrounding quotes from a raw `use parent`/`use base` argument token.
    ///
    /// The parser stores args as raw token text (e.g., `'Animal'` or `"Animal"`).
    /// This removes a single layer of matching `'` or `"` delimiters.
    fn strip_arg_quotes(raw: &str) -> &str {
        let s = raw.trim();
        if s.len() >= 2 {
            if (s.starts_with('\'') && s.ends_with('\''))
                || (s.starts_with('"') && s.ends_with('"'))
            {
                return &s[1..s.len() - 1];
            }
        }
        s
    }
}

enum ImplementationTarget {
    Package(String),
    Method {
        package: String,
        method: String,
    },
    #[allow(dead_code)]
    BlessedType(String),
}
