//! Type definition support for Perl LSP
//!
//! This module provides go-to-type-definition functionality,
//! finding the type/class definition for variables and references.

use crate::ast::{Node, NodeKind};
use crate::uri::parse_uri;
use lsp_types::{LocationLink, Position, Range};
use std::collections::HashMap;

/// Provides go-to-type-definition functionality for Perl code.
///
/// Finds and locates type/class definitions for variables and references,
/// enabling LSP clients to navigate to the source of type definitions.
pub struct TypeDefinitionProvider;

impl TypeDefinitionProvider {
    /// Creates a new type definition provider instance.
    pub fn new() -> Self {
        Self
    }

    /// Find type definition for a position in the AST
    pub fn find_type_definition(
        &self,
        ast: &Node,
        line: u32,
        character: u32,
        uri: &str,
        documents: &HashMap<String, String>,
    ) -> Option<Vec<LocationLink>> {
        // Find the node at the given position
        let target_node = self.find_node_at_position(ast, line, character)?;

        // Get the type name from the node
        let type_name = self.extract_type_name(&target_node)?;

        // Get source text for position conversion
        let source_text = documents.get(uri)?;

        // Find the package/class definition
        self.find_package_definition(ast, &type_name, uri, source_text)
    }

    /// Extract type name from a node
    fn extract_type_name(&self, node: &Node) -> Option<String> {
        match &node.kind {
            // Method call: $obj->method
            NodeKind::MethodCall { object, .. } => {
                // Try to infer the type of the object
                self.infer_object_type(object)
            }
            // Package method: Package::method or Package->method
            NodeKind::Identifier { name } if name.contains("::") => {
                let parts: Vec<&str> = name.split("::").collect();
                if parts.len() >= 2 {
                    // Get the package name (everything except the last part)
                    Some(parts[..parts.len() - 1].join("::"))
                } else {
                    None
                }
            }
            // Constructor: new Package or Package->new
            NodeKind::Binary { op, left, right } if op == "->" => {
                if let NodeKind::Identifier { name: pkg } = &left.kind {
                    if let NodeKind::Identifier { name: method } = &right.kind {
                        if method == "new" {
                            return Some(pkg.clone());
                        }
                    }
                }
                None
            }
            // Blessed reference: bless $ref, 'Package'
            NodeKind::FunctionCall { name, args } if name == "bless" => {
                if args.len() >= 2 {
                    // Second argument is the package name
                    match &args[1].kind {
                        NodeKind::String { value, .. } => Some(value.clone()),
                        NodeKind::Identifier { name } => Some(name.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            // ISA check: $obj isa Package
            NodeKind::Binary { op, right, .. } if op == "isa" => match &right.kind {
                NodeKind::String { value, .. } => Some(value.clone()),
                NodeKind::Identifier { name } => Some(name.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// Try to infer the type of an object from its declaration or assignment
    fn infer_object_type(&self, object: &Node) -> Option<String> {
        match &object.kind {
            NodeKind::Variable { name, .. } => {
                // Would need to track variable types through analysis
                // For now, try common patterns like $self
                if name == "$self" || name == "$this" {
                    // Would need to find the enclosing package
                    None
                } else {
                    None
                }
            }
            // Direct constructor call
            NodeKind::FunctionCall { name, .. } if name == "new" => {
                // The package should be in the parent context
                None
            }
            _ => None,
        }
    }

    /// Find package definition in the AST
    fn find_package_definition(
        &self,
        ast: &Node,
        package_name: &str,
        uri: &str,
        source_text: &str,
    ) -> Option<Vec<LocationLink>> {
        let mut locations = Vec::new();
        self.find_package_in_node(ast, package_name, uri, source_text, &mut locations);

        if !locations.is_empty() { Some(locations) } else { None }
    }

    /// Recursively find package definitions
    fn find_package_in_node(
        &self,
        node: &Node,
        package_name: &str,
        uri: &str,
        source_text: &str,
        locations: &mut Vec<LocationLink>,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } if name == package_name => {
                // Convert byte offsets to UTF-16 line/column
                let (start_line, start_col) =
                    crate::position::offset_to_utf16_line_col(source_text, node.location.start);
                let (end_line, end_col) =
                    crate::position::offset_to_utf16_line_col(source_text, node.location.end);

                // Create typed LocationLink for better UI experience
                locations.push(LocationLink {
                    origin_selection_range: None, // Could be filled with the reference range
                    target_uri: parse_uri(uri),
                    target_range: Range::new(
                        Position::new(start_line, start_col),
                        Position::new(end_line, end_col),
                    ),
                    target_selection_range: Range::new(
                        Position::new(start_line, start_col),
                        Position::new(end_line, end_col),
                    ),
                });
            }
            _ => {}
        }

        // Recurse into children based on node type
        self.visit_children(node, |child| {
            self.find_package_in_node(child, package_name, uri, source_text, locations);
        });
    }

    /// Helper to visit children of a node
    fn visit_children<F>(&self, node: &Node, mut f: F)
    where
        F: FnMut(&Node),
    {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }
            NodeKind::Package { block, .. } => {
                if let Some(b) = block {
                    f(b);
                }
            }
            _ => {
                // Most other nodes don't have children we need to traverse for packages
            }
        }
    }

    /// Find node at the given position
    fn find_node_at_position(&self, node: &Node, _line: u32, _character: u32) -> Option<Node> {
        // Simplified - would need proper offset calculation from line/column
        // For now, just return the first matching node type
        if matches!(
            &node.kind,
            NodeKind::Package { .. } | NodeKind::Identifier { .. } | NodeKind::FunctionCall { .. }
        ) {
            // Check children for more specific match
            let mut result = None;
            self.visit_children(node, |child| {
                if result.is_none() {
                    if let Some(found) = self.find_node_at_position(child, _line, _character) {
                        result = Some(found);
                    }
                }
            });

            if result.is_some() {
                return result;
            }

            // No child contains the position, return this node
            return Some(node.clone());
        }

        None
    }
}

impl Default for TypeDefinitionProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_find_package_definition() {
        let code = r#"
package MyClass;

sub new {
    my $class = shift;
    bless {}, $class;
}

package main;

my $obj = MyClass->new();
$obj->method();
"#;
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("Failed to parse");

        let provider = TypeDefinitionProvider::new();
        let uri = "file:///test.pl";

        // Test finding MyClass definition
        let locations = provider.find_package_definition(&ast, "MyClass", uri, code);
        assert!(locations.is_some());
        let locs = locations.unwrap();
        assert_eq!(locs.len(), 1);
    }

    #[test]
    fn test_extract_type_from_constructor() {
        let code = "my $obj = Package::Name->new();";
        let mut parser = Parser::new(code);
        let _ast = parser.parse().expect("Failed to parse");

        let _provider = TypeDefinitionProvider::new();

        // Would need to traverse to find the right node
        // This is a simplified test
    }
}
