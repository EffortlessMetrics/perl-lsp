//! Type definition support for Perl LSP
//!
//! This module provides go-to-type-definition functionality,
//! finding the type/class definition for variables and references.

use crate::ast::{Node, NodeKind};
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct TypeDefinitionProvider;

impl TypeDefinitionProvider {
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
        _documents: &HashMap<String, String>,
    ) -> Option<Vec<Value>> {
        // Find the node at the given position
        let target_node = self.find_node_at_position(ast, line, character)?;
        
        // Get the type name from the node
        let type_name = self.extract_type_name(&target_node)?;
        
        // Find the package/class definition
        self.find_package_definition(ast, &type_name, uri)
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
            NodeKind::Binary { op, right, .. } if op == "isa" => {
                match &right.kind {
                    NodeKind::String { value, .. } => Some(value.clone()),
                    NodeKind::Identifier { name } => Some(name.clone()),
                    _ => None,
                }
            }
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
    ) -> Option<Vec<Value>> {
        let mut locations = Vec::new();
        self.find_package_in_node(ast, package_name, uri, &mut locations);
        
        if !locations.is_empty() {
            Some(locations)
        } else {
            None
        }
    }

    /// Recursively find package definitions
    fn find_package_in_node(
        &self,
        node: &Node,
        package_name: &str,
        uri: &str,
        locations: &mut Vec<Value>,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } if name == package_name => {
                // For now, use a dummy range - would need proper offset-to-position conversion
                locations.push(json!({
                    "uri": uri,
                    "range": {
                        "start": {
                            "line": 0,
                            "character": 0,
                        },
                        "end": {
                            "line": 0,
                            "character": 0,
                        },
                    },
                }));
            }
            _ => {}
        }

        // Recurse into children based on node type
        self.visit_children(node, |child| {
            self.find_package_in_node(child, package_name, uri, locations);
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
        if matches!(&node.kind, NodeKind::Package { .. } | NodeKind::Identifier { .. } | NodeKind::FunctionCall { .. })
        {
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
        let locations = provider.find_package_definition(&ast, "MyClass", uri);
        assert!(locations.is_some());
        let locs = locations.unwrap();
        assert_eq!(locs.len(), 1);
    }

    #[test]
    fn test_extract_type_from_constructor() {
        let code = "my $obj = Package::Name->new();";
        let mut parser = Parser::new(code);
        let ast = parser.parse().expect("Failed to parse");
        
        let provider = TypeDefinitionProvider::new();
        
        // Would need to traverse to find the right node
        // This is a simplified test
    }
}