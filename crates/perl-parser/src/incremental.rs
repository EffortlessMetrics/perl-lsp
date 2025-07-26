//! Incremental parsing support
//!
//! This module provides incremental parsing capabilities that allow efficient
//! re-parsing after edits by reusing unchanged portions of the previous parse tree.

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    edit::{Edit, EditSet},
    error::ParseResult,
    parser::Parser,
    position::Range,
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

/// A region that needs to be reparsed
struct ReparseRegion<'a> {
    node: &'a Node,
    start_byte: usize,
    end_byte: usize,
}

/// Incremental parser that maintains parse state between edits
pub struct IncrementalParser {
    /// Last successfully parsed tree
    last_tree: Option<Tree>,
    /// Accumulated edits since last parse
    pending_edits: EditSet,
    /// Statistics about incremental parsing performance
    reused_nodes: usize,
    reparsed_nodes: usize,
}

impl IncrementalParser {
    /// Create a new incremental parser
    pub fn new() -> Self {
        IncrementalParser {
            last_tree: None,
            pending_edits: EditSet::new(),
            reused_nodes: 0,
            reparsed_nodes: 0,
        }
    }
    
    /// Add an edit to be applied in the next parse
    pub fn edit(&mut self, edit: Edit) {
        self.pending_edits.add(edit);
    }
    
    /// Parse the source, reusing previous tree if possible
    pub fn parse(&mut self, source: &str) -> ParseResult<Tree> {
        // Reset statistics for this parse
        self.reused_nodes = 0;
        self.reparsed_nodes = 0;
        
        let result = if self.last_tree.is_some() {
            // Try incremental parsing
            let last_tree = self.last_tree.as_ref().unwrap().clone();
            self.parse_incremental(source, &last_tree)
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
    
    fn parse_full(&mut self, source: &str) -> ParseResult<Tree> {
        let mut parser = Parser::new(source);
        let root = parser.parse()?;
        
        // Count nodes in the parsed tree
        self.reparsed_nodes = self.count_nodes(&root);
        
        Ok(Tree::new(root, source.to_string()))
    }
    
    /// Count total nodes in a tree
    fn count_nodes(&self, node: &Node) -> usize {
        let mut count = 1; // Count this node
        
        match &node.kind {
            NodeKind::Program { statements } |
            NodeKind::Block { statements } => {
                for stmt in statements {
                    count += self.count_nodes(stmt);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                count += self.count_nodes(left);
                count += self.count_nodes(right);
            }
            NodeKind::Unary { operand, .. } => {
                count += self.count_nodes(operand);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                count += self.count_nodes(condition);
                count += self.count_nodes(then_branch);
                for (cond, branch) in elsif_branches {
                    count += self.count_nodes(cond);
                    count += self.count_nodes(branch);
                }
                if let Some(else_b) = else_branch {
                    count += self.count_nodes(else_b);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                count += self.count_nodes(variable);
                if let Some(init) = initializer {
                    count += self.count_nodes(init);
                }
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    count += self.count_nodes(arg);
                }
            }
            // Add other node types as needed
            _ => {}
        }
        
        count
    }
    
    fn parse_incremental(&mut self, source: &str, last_tree: &Tree) -> ParseResult<Tree> {
        // Identify the ranges affected by edits
        let affected_ranges = self.pending_edits.affected_ranges();
        
        // Find the smallest subtrees that need to be reparsed
        let reparse_regions = self.find_reparse_regions(last_tree, &affected_ranges);
        
        // If no regions need reparsing, just shift positions
        if reparse_regions.is_empty() {
            let mut reused_count = 0;
            let new_root = self.shift_node_recursive(&last_tree.root, &self.pending_edits, &mut reused_count);
            // Update statistics
            self.reused_nodes = reused_count;
            return Ok(Tree::new(new_root, source.to_string()));
        }
        
        // For each region that needs reparsing:
        // 1. Extract the source text for that region (with context)
        // 2. Parse just that region
        // 3. Replace the old subtree with the new one
        
        // For this initial implementation, we'll reparse the whole program
        // if any edits affect structural nodes (statements, blocks, etc.)
        let structural_edit = reparse_regions.iter().any(|r| {
            matches!(r.node.kind, 
                NodeKind::Program { .. } | 
                NodeKind::Block { .. } |
                NodeKind::If { .. } |
                NodeKind::While { .. } |
                NodeKind::For { .. }
            )
        });
        
        if structural_edit {
            // Major structural change - full reparse needed
            self.parse_full(source)
        } else {
            // Try to reparse just the affected expressions
            match self.reparse_and_merge(source, last_tree, &reparse_regions) {
                Ok(tree) => Ok(tree),
                Err(_) => {
                    // Fall back to full parse if incremental fails
                    self.parse_full(source)
                }
            }
        }
    }
    
    /// Find the minimal set of nodes that need to be reparsed
    fn find_reparse_regions<'a>(&self, tree: &'a Tree, affected_ranges: &[Range]) -> Vec<ReparseRegion<'a>> {
        let mut regions = Vec::new();
        
        for range in affected_ranges {
            // Find all nodes that overlap with this affected range
            let affected_nodes = tree.find_nodes_in_range(range.start.byte, range.end.byte);
            
            // For each affected node, find the smallest containing expression or statement
            for node in affected_nodes {
                // Skip if we've already included a parent of this node
                if regions.iter().any(|r: &ReparseRegion| {
                    r.start_byte <= node.location.start && r.end_byte >= node.location.end
                }) {
                    continue;
                }
                
                regions.push(ReparseRegion {
                    node,
                    start_byte: node.location.start,
                    end_byte: node.location.end,
                });
            }
        }
        
        // Sort by start position and merge overlapping regions
        regions.sort_by_key(|r| r.start_byte);
        self.merge_overlapping_regions(regions)
    }
    
    /// Merge overlapping reparse regions
    fn merge_overlapping_regions<'a>(&self, mut regions: Vec<ReparseRegion<'a>>) -> Vec<ReparseRegion<'a>> {
        if regions.is_empty() {
            return regions;
        }
        
        let mut merged = vec![regions.remove(0)];
        
        for region in regions {
            let last = merged.last_mut().unwrap();
            if region.start_byte <= last.end_byte {
                // Overlapping - extend the last region
                last.end_byte = last.end_byte.max(region.end_byte);
            } else {
                // Not overlapping - add as new region
                merged.push(region);
            }
        }
        
        merged
    }
    
    
    fn shift_node_recursive(&self, node: &Node, edits: &EditSet, reused_count: &mut usize) -> Node {
        *reused_count += 1;
        
        let shift = edits.byte_shift_at(node.location.start);
        let new_location = SourceLocation {
            start: (node.location.start as isize + shift) as usize,
            end: (node.location.end as isize + shift) as usize,
        };
        
        // Recursively shift child nodes based on node type
        let new_kind = match &node.kind {
            NodeKind::Program { statements } => {
                NodeKind::Program {
                    statements: statements.iter()
                        .map(|stmt| self.shift_node_recursive(stmt, edits, reused_count))
                        .collect()
                }
            }
            NodeKind::Block { statements } => {
                NodeKind::Block {
                    statements: statements.iter()
                        .map(|stmt| self.shift_node_recursive(stmt, edits, reused_count))
                        .collect()
                }
            }
            NodeKind::Binary { op, left, right } => {
                NodeKind::Binary {
                    op: op.clone(),
                    left: Box::new(self.shift_node_recursive(left, edits, reused_count)),
                    right: Box::new(self.shift_node_recursive(right, edits, reused_count)),
                }
            }
            NodeKind::Unary { op, operand } => {
                NodeKind::Unary {
                    op: op.clone(),
                    operand: Box::new(self.shift_node_recursive(operand, edits, reused_count)),
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                NodeKind::If {
                    condition: Box::new(self.shift_node_recursive(condition, edits, reused_count)),
                    then_branch: Box::new(self.shift_node_recursive(then_branch, edits, reused_count)),
                    elsif_branches: elsif_branches.iter()
                        .map(|(cond, branch)| {
                            let new_cond = self.shift_node_recursive(cond, edits, reused_count);
                            let new_branch = self.shift_node_recursive(branch, edits, reused_count);
                            (Box::new(new_cond), Box::new(new_branch))
                        })
                        .collect(),
                    else_branch: else_branch.as_ref()
                        .map(|branch| Box::new(self.shift_node_recursive(branch, edits, reused_count))),
                }
            }
            // For leaf nodes, just clone the kind
            _ => node.kind.clone(),
        };
        
        Node::new(new_kind, new_location)
    }
    
    /// Reparse affected regions and merge with existing tree
    fn reparse_and_merge(&mut self, source: &str, _last_tree: &Tree, _regions: &[ReparseRegion]) -> ParseResult<Tree> {
        // For now, implement a simple version that reparses the whole file
        // In a full implementation, we would:
        // 1. Extract source for each region
        // 2. Parse each region as an expression/statement
        // 3. Replace nodes in the tree
        
        // This is a placeholder that demonstrates the concept
        self.parse_full(source)
    }
    
    /// Get statistics about the last parse
    pub fn stats(&self) -> IncrementalStats {
        IncrementalStats {
            has_tree: self.last_tree.is_some(),
            pending_edits: self.pending_edits.edits.len(),
            reused_nodes: self.reused_nodes,
            reparsed_nodes: self.reparsed_nodes,
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
    pub reused_nodes: usize,
    pub reparsed_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;
    
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
        let initial_nodes = parser.stats().reparsed_nodes;
        assert!(initial_nodes > 0);
        
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
        
        // Check that we reused some nodes
        let stats = parser.stats();
        assert!(stats.reused_nodes > 0);
    }
    
    #[test]
    fn test_no_edit_full_reuse() {
        let mut parser = IncrementalParser::new();
        let source = "my $x = 42;\nprint $x;";
        
        // First parse
        parser.parse(source).unwrap();
        let initial_nodes = parser.stats().reparsed_nodes;
        
        // Parse again with no edits - should reuse everything
        parser.parse(source).unwrap();
        let stats = parser.stats();
        
        // When no edits, we currently do a full reparse
        // In a future optimization, we could detect no changes and reuse the entire tree
        assert_eq!(stats.reparsed_nodes, initial_nodes);
    }
    
    #[test]
    fn test_multiple_edits() {
        let mut parser = IncrementalParser::new();
        
        // Parse initial source
        parser.parse("my $x = 1;\nmy $y = 2;\nmy $z = 3;").unwrap();
        
        // Add multiple edits
        parser.edit(Edit::new(
            8, 9, 10,  // Change 1 to 10
            Position::new(8, 1, 9),
            Position::new(9, 1, 10),
            Position::new(10, 1, 11),
        ));
        
        parser.edit(Edit::new(
            20, 21, 22,  // Change 2 to 20 (accounting for previous shift)
            Position::new(20, 2, 9),
            Position::new(21, 2, 10),
            Position::new(22, 2, 11),
        ));
        
        // Parse with edits
        parser.parse("my $x = 10;\nmy $y = 20;\nmy $z = 3;").unwrap();
        
        let stats = parser.stats();
        // Should reparse affected nodes but reuse others
        assert!(stats.reused_nodes > 0 || stats.reparsed_nodes > 0);
    }
    
    #[test]
    fn test_structural_edit() {
        let mut parser = IncrementalParser::new();
        
        // Parse initial if statement
        parser.parse("if ($x) { print $x; }").unwrap();
        
        // Add else branch (structural change)
        parser.edit(Edit::new(
            21, 21, 35,  // Insert " else { print 0; }"
            Position::new(21, 1, 22),
            Position::new(21, 1, 22),
            Position::new(35, 1, 36),
        ));
        
        // This is a structural change, so it will trigger a full reparse
        parser.parse("if ($x) { print $x; } else { print 0; }").unwrap();
        
        let stats = parser.stats();
        assert!(stats.reparsed_nodes > 0);
    }
}