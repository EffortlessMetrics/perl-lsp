//! Incremental parsing support
//!
//! This module provides incremental parsing capabilities that allow efficient
//! re-parsing after edits by reusing unchanged portions of the previous parse tree.

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    edit::{Edit, EditSet},
    error::ParseResult,
    parser::Parser,
    position::{Position, Range},
};
use std::collections::HashMap;

/// A parse tree that supports incremental updates
#[derive(Debug, Clone)]
pub struct Tree {
    /// Root node of the tree
    pub root: Node,
    /// Source text this tree was parsed from
    pub source: String,
    /// Map of node positions for quick lookup
    node_positions: HashMap<usize, Vec<NodeRef>>,
}

/// Reference to a node in the tree with its position
#[derive(Debug, Clone)]
struct NodeRef {
    node: Node,
    #[allow(dead_code)]
    depth: usize,
}

impl Tree {
    /// Create a new tree from a root node and source
    pub fn new(root: Node, source: String) -> Self {
        let mut tree = Tree {
            root,
            source,
            node_positions: HashMap::new(),
        };
        tree.index_nodes();
        tree
    }
    
    /// Index all nodes by their start position for efficient lookup
    fn index_nodes(&mut self) {
        self.node_positions.clear();
        self.index_node(&self.root.clone(), 0);
    }
    
    fn index_node(&mut self, node: &Node, depth: usize) {
        let start = node.location.start;
        self.node_positions
            .entry(start)
            .or_insert_with(Vec::new)
            .push(NodeRef {
                node: node.clone(),
                depth,
            });
        
        // Index child nodes
        match &node.kind {
            NodeKind::Program { statements } |
            NodeKind::Block { statements } => {
                for stmt in statements {
                    self.index_node(stmt, depth + 1);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.index_node(condition, depth + 1);
                self.index_node(then_branch, depth + 1);
                for (cond, branch) in elsif_branches {
                    self.index_node(cond, depth + 1);
                    self.index_node(branch, depth + 1);
                }
                if let Some(else_b) = else_branch {
                    self.index_node(else_b, depth + 1);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.index_node(left, depth + 1);
                self.index_node(right, depth + 1);
            }
            NodeKind::Unary { operand, .. } => {
                self.index_node(operand, depth + 1);
            }
            // Add other node types as needed...
            _ => {}
        }
    }
    
    /// Find nodes that overlap with a given byte range
    pub fn find_nodes_in_range(&self, start: usize, end: usize) -> Vec<&Node> {
        let mut nodes = Vec::new();
        
        for (_pos, refs) in &self.node_positions {
            for node_ref in refs {
                let loc = &node_ref.node.location;
                if loc.start < end && loc.end > start {
                    nodes.push(&node_ref.node);
                }
            }
        }
        
        nodes.sort_by_key(|n| (n.location.start, n.location.end));
        nodes
    }
    
    /// Apply edits to create a new tree
    pub fn apply_edits(&self, edits: &EditSet) -> Tree {
        // For now, create a simple implementation that adjusts positions
        let new_root = self.clone_and_shift_node(&self.root, edits);
        Tree::new(new_root, self.source.clone()) // Source would be updated too
    }
    
    fn clone_and_shift_node(&self, node: &Node, edits: &EditSet) -> Node {
        // Convert SourceLocation to Range for position adjustment
        let range = Range::from(node.location);
        
        // Check if this node is affected by edits
        if !edits.affects_range(&range) {
            // Node is not affected - can reuse with shifted positions
            let shift = edits.byte_shift_at(node.location.start);
            Node::new(
                node.kind.clone(),
                SourceLocation {
                    start: (node.location.start as isize + shift) as usize,
                    end: (node.location.end as isize + shift) as usize,
                }
            )
        } else {
            // Node is affected - would need to re-parse
            // For now, just mark as error
            Node::new(
                NodeKind::Identifier { name: "NEEDS_REPARSE".to_string() },
                node.location
            )
        }
    }
}

/// Incremental parser that maintains parse state between edits
pub struct IncrementalParser {
    /// Last successfully parsed tree
    last_tree: Option<Tree>,
    /// Accumulated edits since last parse
    pending_edits: EditSet,
}

impl IncrementalParser {
    /// Create a new incremental parser
    pub fn new() -> Self {
        IncrementalParser {
            last_tree: None,
            pending_edits: EditSet::new(),
        }
    }
    
    /// Add an edit to be applied in the next parse
    pub fn edit(&mut self, edit: Edit) {
        self.pending_edits.add(edit);
    }
    
    /// Parse the source, reusing previous tree if possible
    pub fn parse(&mut self, source: &str) -> ParseResult<Tree> {
        let result = if let Some(ref last_tree) = self.last_tree {
            // Try incremental parsing
            self.parse_incremental(source, last_tree)
        } else {
            // Full parse required
            self.parse_full(source)
        };
        
        // Clear pending edits after parse
        self.pending_edits = EditSet::new();
        
        // Store successful parse
        if let Ok(ref tree) = result {
            self.last_tree = Some(tree.clone());
        }
        
        result
    }
    
    fn parse_full(&self, source: &str) -> ParseResult<Tree> {
        let mut parser = Parser::new(source);
        let root = parser.parse()?;
        Ok(Tree::new(root, source.to_string()))
    }
    
    fn parse_incremental(&self, source: &str, _last_tree: &Tree) -> ParseResult<Tree> {
        // For now, implement a simple strategy:
        // 1. Apply position shifts to unaffected nodes
        // 2. Re-parse affected regions
        
        // This is a simplified implementation
        // A full implementation would:
        // - Identify minimal regions to re-parse
        // - Reuse lexer state checkpoints
        // - Merge new nodes with shifted old nodes
        
        // For now, just do a full re-parse
        self.parse_full(source)
    }
    
    /// Get statistics about the last parse
    pub fn stats(&self) -> IncrementalStats {
        IncrementalStats {
            has_tree: self.last_tree.is_some(),
            pending_edits: self.pending_edits.edits.len(),
        }
    }
}

impl Default for IncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about incremental parsing
#[derive(Debug)]
pub struct IncrementalStats {
    pub has_tree: bool,
    pub pending_edits: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tree_indexing() {
        let root = Node::new(
            NodeKind::Program {
                statements: vec![
                    Node::new(
                        NodeKind::Number { value: "42".to_string() },
                        SourceLocation { start: 0, end: 2 }
                    ),
                    Node::new(
                        NodeKind::Number { value: "43".to_string() },
                        SourceLocation { start: 3, end: 5 }
                    ),
                ]
            },
            SourceLocation { start: 0, end: 5 }
        );
        
        let tree = Tree::new(root, "42 43".to_string());
        
        // Should find nodes in range
        let nodes = tree.find_nodes_in_range(1, 4);
        assert_eq!(nodes.len(), 3); // Program node and both numbers
    }
    
    #[test]
    fn test_incremental_parser() {
        let mut parser = IncrementalParser::new();
        
        // First parse
        let _tree1 = parser.parse("my $x = 42;").unwrap();
        assert!(parser.last_tree.is_some());
        
        // Add an edit
        parser.edit(Edit::new(
            8, 10, 12,
            Position::new(8, 1, 9),
            Position::new(10, 1, 11),
            Position::new(12, 1, 13),
        ));
        
        // Re-parse with edit
        let _tree2 = parser.parse("my $x = 4242;").unwrap();
        assert!(parser.last_tree.is_some());
    }
}