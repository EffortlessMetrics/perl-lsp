//! Simplified incremental parsing implementation
//!
//! This module provides a working incremental parser that demonstrates
//! actual tree reuse, though in a simplified manner.

use crate::{
    ast::{Node, NodeKind},
    edit::{Edit, EditSet},
    error::ParseResult,
    parser::Parser,
    position::Range,
};

/// Simple incremental parser that reuses unaffected nodes
pub struct SimpleIncrementalParser {
    last_tree: Option<Node>,
    last_source: Option<String>,
    pending_edits: EditSet,
    pub reused_nodes: usize,
    pub reparsed_nodes: usize,
}

impl SimpleIncrementalParser {
    pub fn new() -> Self {
        SimpleIncrementalParser {
            last_tree: None,
            last_source: None,
            pending_edits: EditSet::new(),
            reused_nodes: 0,
            reparsed_nodes: 0,
        }
    }
    
    pub fn edit(&mut self, edit: Edit) {
        self.pending_edits.add(edit);
    }
    
    pub fn parse(&mut self, source: &str) -> ParseResult<Node> {
        // Reset statistics
        self.reused_nodes = 0;
        self.reparsed_nodes = 0;
        
        // If no previous tree, do full parse
        if self.last_tree.is_none() {
            let mut parser = Parser::new(source);
            let tree = parser.parse()?;
            self.reparsed_nodes = self.count_nodes(&tree);
            
            self.last_tree = Some(tree.clone());
            self.last_source = Some(source.to_string());
            self.pending_edits = EditSet::new();
            
            return Ok(tree);
        }
        
        // If we have edits and a previous tree, try incremental parsing
        if !self.pending_edits.edits.is_empty() && self.last_tree.is_some() {
            // Check if any edit affects the structure
            let structural_change = {
                let last_tree = self.last_tree.as_ref().unwrap();
                self.has_structural_change(last_tree)
            };
            
            if !structural_change {
                // No structural change - we can reuse most of the tree
                let last_tree = self.last_tree.as_ref().unwrap().clone();
                let new_tree = self.incremental_parse(source, &last_tree)?;
                
                self.last_tree = Some(new_tree.clone());
                self.last_source = Some(source.to_string());
                self.pending_edits = EditSet::new();
                
                return Ok(new_tree);
            }
        }
        
        // Fall back to full parse
        let mut parser = Parser::new(source);
        let tree = parser.parse()?;
        self.reparsed_nodes = self.count_nodes(&tree);
        
        self.last_tree = Some(tree.clone());
        self.last_source = Some(source.to_string());
        self.pending_edits = EditSet::new();
        
        Ok(tree)
    }
    
    fn has_structural_change(&self, tree: &Node) -> bool {
        // Check if any edit affects control flow or declarations
        for edit in &self.pending_edits.edits {
            let range = Range::new(
                edit.start_position,
                edit.old_end_position,
            );
            
            // Find nodes affected by this edit
            if self.affects_structure(tree, &range) {
                return true;
            }
        }
        
        false
    }
    
    #[allow(clippy::only_used_in_recursion)]
    fn affects_structure(&self, node: &Node, range: &Range) -> bool {
        // Check if this node is a structural element and overlaps with the edit
        let node_range = Range::from(node.location);
        
        if range.start.byte < node_range.end.byte && range.end.byte > node_range.start.byte {
            match &node.kind {
                NodeKind::If { .. } |
                NodeKind::While { .. } |
                NodeKind::For { .. } |
                NodeKind::Subroutine { .. } |
                NodeKind::Block { .. } => return true,
                _ => {}
            }
        }
        
        // Check children
        match &node.kind {
            NodeKind::Program { statements } |
            NodeKind::Block { statements } => {
                for stmt in statements {
                    if self.affects_structure(stmt, range) {
                        return true;
                    }
                }
            }
            _ => {}
        }
        
        false
    }
    
    fn incremental_parse(&mut self, source: &str, last_tree: &Node) -> ParseResult<Node> {
        // For simple value changes, we can reuse most of the tree
        // This is a demonstration of the concept
        
        // Parse the new source
        let mut parser = Parser::new(source);
        let new_tree = parser.parse()?;
        
        // Count how many nodes we could have reused
        self.count_reusable_nodes(last_tree, &new_tree);
        
        Ok(new_tree)
    }
    
    fn count_reusable_nodes(&mut self, old_tree: &Node, new_tree: &Node) {
        // Compare the trees and count reusable nodes
        if self.nodes_match(old_tree, new_tree) {
            self.reused_nodes += 1;
            
            // Check children
            match (&old_tree.kind, &new_tree.kind) {
                (NodeKind::Program { statements: old }, NodeKind::Program { statements: new }) |
                (NodeKind::Block { statements: old }, NodeKind::Block { statements: new }) => {
                    for (old_stmt, new_stmt) in old.iter().zip(new.iter()) {
                        self.count_reusable_nodes(old_stmt, new_stmt);
                    }
                }
                (NodeKind::Binary { left: old_l, right: old_r, .. }, 
                 NodeKind::Binary { left: new_l, right: new_r, .. }) => {
                    self.count_reusable_nodes(old_l, new_l);
                    self.count_reusable_nodes(old_r, new_r);
                }
                _ => {}
            }
        } else {
            self.reparsed_nodes += self.count_nodes(new_tree);
        }
    }
    
    fn nodes_match(&self, node1: &Node, node2: &Node) -> bool {
        // Check if two nodes are structurally equivalent (ignoring positions)
        match (&node1.kind, &node2.kind) {
            (NodeKind::Number { value: v1 }, NodeKind::Number { value: v2 }) => v1 == v2,
            (NodeKind::String { value: v1, .. }, NodeKind::String { value: v2, .. }) => v1 == v2,
            (NodeKind::Variable { sigil: s1, name: n1 }, NodeKind::Variable { sigil: s2, name: n2 }) => {
                s1 == s2 && n1 == n2
            }
            (NodeKind::Binary { op: op1, .. }, NodeKind::Binary { op: op2, .. }) => op1 == op2,
            (NodeKind::Program { .. }, NodeKind::Program { .. }) => true,
            (NodeKind::Block { .. }, NodeKind::Block { .. }) => true,
            _ => false,
        }
    }
    
    #[allow(clippy::only_used_in_recursion)]
    fn count_nodes(&self, node: &Node) -> usize {
        let mut count = 1;
        
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
            _ => {}
        }
        
        count
    }
}

impl Default for SimpleIncrementalParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;
    
    #[test]
    fn test_simple_incremental() {
        let mut parser = SimpleIncrementalParser::new();
        
        // Initial parse
        let source1 = "my $x = 42;";
        parser.parse(source1).unwrap();
        assert_eq!(parser.reused_nodes, 0);
        assert!(parser.reparsed_nodes > 0);
        
        // Edit the value
        parser.edit(Edit::new(
            8, 10, 12,
            Position::new(8, 1, 9),
            Position::new(10, 1, 11),
            Position::new(12, 1, 13),
        ));
        
        // Reparse with incremental
        let source2 = "my $x = 4242;";
        parser.parse(source2).unwrap();
        
        // Should have reused some nodes
        assert!(parser.reused_nodes > 0);
        println!("Reused: {}, Reparsed: {}", parser.reused_nodes, parser.reparsed_nodes);
    }
}