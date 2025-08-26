//! Working incremental parsing implementation with actual tree reuse
//!
//! This module provides a functional incremental parser that demonstrates
//! real tree reuse for non-structural edits.

use crate::{
    ast::{Node, NodeKind, SourceLocation},
    edit::{Edit, EditSet},
    error::ParseResult,
    parser::Parser,
};
use std::collections::HashMap;

/// A parse tree with incremental parsing support
#[derive(Debug, Clone)]
pub struct IncrementalTree {
    pub root: Node,
    pub source: String,
    /// Maps byte positions to nodes for efficient lookup
    node_map: HashMap<usize, Vec<Node>>,
}

impl IncrementalTree {
    /// Create a new incremental tree
    pub fn new(root: Node, source: String) -> Self {
        let mut tree = IncrementalTree { root, source, node_map: HashMap::new() };
        tree.build_node_map();
        tree
    }

    /// Build a map of byte positions to nodes
    fn build_node_map(&mut self) {
        self.node_map.clear();
        self.map_node(&self.root.clone());
    }

    fn map_node(&mut self, node: &Node) {
        // Map start position to node
        self.node_map.entry(node.location.start).or_default().push(node.clone());

        // Recursively map child nodes
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.map_node(stmt);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                self.map_node(variable);
                if let Some(init) = initializer {
                    self.map_node(init);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                self.map_node(left);
                self.map_node(right);
            }
            NodeKind::Unary { operand, .. } => {
                self.map_node(operand);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    self.map_node(arg);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                self.map_node(condition);
                self.map_node(then_branch);
                for (cond, branch) in elsif_branches {
                    self.map_node(cond);
                    self.map_node(branch);
                }
                if let Some(branch) = else_branch {
                    self.map_node(branch);
                }
            }
            _ => {}
        }
    }

    /// Find the smallest node containing the given byte range
    pub fn find_containing_node(&self, start: usize, end: usize) -> Option<&Node> {
        let mut smallest: Option<&Node> = None;
        let mut smallest_size = usize::MAX;

        // Check all nodes
        for nodes in self.node_map.values() {
            for node in nodes {
                if node.location.start <= start && node.location.end >= end {
                    let size = node.location.end - node.location.start;
                    if size < smallest_size {
                        smallest = Some(node);
                        smallest_size = size;
                    }
                }
            }
        }

        smallest
    }
}

/// Incremental parser with working tree reuse
pub struct IncrementalParserV2 {
    last_tree: Option<IncrementalTree>,
    pending_edits: EditSet,
    pub reused_nodes: usize,
    pub reparsed_nodes: usize,
}

#[allow(dead_code)]
impl IncrementalParserV2 {
    pub fn new() -> Self {
        IncrementalParserV2 {
            last_tree: None,
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

        // Try incremental parsing if we have a previous tree and edits
        if self.last_tree.is_some() && !self.pending_edits.edits.is_empty() {
            let last_tree = self.last_tree.as_ref().unwrap().clone();
            // Check if we can do incremental parsing
            if let Some(new_tree) = self.try_incremental_parse(source, &last_tree) {
                self.last_tree = Some(IncrementalTree::new(new_tree.clone(), source.to_string()));
                self.pending_edits = EditSet::new();
                return Ok(new_tree);
            }
        }

        // Fall back to full parse
        self.full_parse(source)
    }

    fn full_parse(&mut self, source: &str) -> ParseResult<Node> {
        let mut parser = Parser::new(source);
        let root = parser.parse()?;

        self.reparsed_nodes = self.count_nodes(&root);
        self.last_tree = Some(IncrementalTree::new(root.clone(), source.to_string()));
        self.pending_edits = EditSet::new();

        Ok(root)
    }

    fn try_incremental_parse(&mut self, source: &str, last_tree: &IncrementalTree) -> Option<Node> {
        // For simple value edits, we can reuse most of the tree
        if self.is_simple_value_edit(last_tree) {
            return self.incremental_parse_simple(source, last_tree);
        }

        // For other cases, fall back to full parse
        None
    }

    fn is_simple_value_edit(&self, tree: &IncrementalTree) -> bool {
        // Check if all edits only affect literal values
        let mut cumulative_shift: isize = 0;
        for edit in &self.pending_edits.edits {
            // Map edit positions back to the original tree coordinates
            let start = (edit.start_byte as isize - cumulative_shift) as usize;
            let end = (edit.old_end_byte as isize - cumulative_shift) as usize;

            let affected_node = tree.find_containing_node(start, end);

            match affected_node {
                Some(node) => match &node.kind {
                    NodeKind::Number { .. } | NodeKind::String { .. } => {
                        if start >= node.location.start && end <= node.location.end {
                            // Edit is entirely within the literal
                            cumulative_shift += edit.byte_shift();
                            continue;
                        } else {
                            return false;
                        }
                    }
                    _ => return false, // Structural edit
                },
                None => return false, // No containing node found
            }
        }

        true
    }

    fn incremental_parse_simple(
        &mut self,
        source: &str,
        last_tree: &IncrementalTree,
    ) -> Option<Node> {
        // For simple value edits, reuse unaffected nodes from the previous tree
        let new_root = self.clone_and_update_node(&last_tree.root, source, &last_tree.source);

        // Update reuse statistics
        self.count_reuse_potential(&last_tree.root, &new_root);

        Some(new_root)
    }

    fn clone_and_update_node(&self, node: &Node, new_source: &str, _old_source: &str) -> Node {
        let shift = self.calculate_shift_at(node.location.start);
        let affected = self.is_node_affected(node);
        let content_delta = if affected { self.calculate_content_delta(node) } else { 0 };
        let new_start = (node.location.start as isize + shift) as usize;
        let new_end = (node.location.end as isize + shift + content_delta) as usize;

        match &node.kind {
            NodeKind::Program { statements } => {
                let new_stmts = statements
                    .iter()
                    .map(|s| self.clone_and_update_node(s, new_source, _old_source))
                    .collect();
                Node::new(
                    NodeKind::Program { statements: new_stmts },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::Block { statements } => {
                let new_stmts = statements
                    .iter()
                    .map(|s| self.clone_and_update_node(s, new_source, _old_source))
                    .collect();
                Node::new(
                    NodeKind::Block { statements: new_stmts },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                let new_var = self.clone_and_update_node(variable, new_source, _old_source);
                let new_init = initializer
                    .as_ref()
                    .map(|i| self.clone_and_update_node(i, new_source, _old_source));
                Node::new(
                    NodeKind::VariableDeclaration {
                        declarator: declarator.clone(),
                        variable: Box::new(new_var),
                        attributes: attributes.clone(),
                        initializer: new_init.map(Box::new),
                    },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::Binary { op, left, right } => {
                let new_left = self.clone_and_update_node(left, new_source, _old_source);
                let new_right = self.clone_and_update_node(right, new_source, _old_source);
                Node::new(
                    NodeKind::Binary {
                        op: op.clone(),
                        left: Box::new(new_left),
                        right: Box::new(new_right),
                    },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::Unary { op, operand } => {
                let new_operand = self.clone_and_update_node(operand, new_source, _old_source);
                Node::new(
                    NodeKind::Unary {
                        op: op.clone(),
                        operand: Box::new(new_operand),
                    },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::FunctionCall { name, args } => {
                let new_args = args
                    .iter()
                    .map(|a| self.clone_and_update_node(a, new_source, _old_source))
                    .collect();
                Node::new(
                    NodeKind::FunctionCall {
                        name: name.clone(),
                        args: new_args,
                    },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let new_condition = self.clone_and_update_node(condition, new_source, _old_source);
                let new_then = self.clone_and_update_node(then_branch, new_source, _old_source);
                let new_elsif: Vec<(Box<Node>, Box<Node>)> = elsif_branches
                    .iter()
                    .map(|(c, b)| {
                        (
                            Box::new(self.clone_and_update_node(c, new_source, _old_source)),
                            Box::new(self.clone_and_update_node(b, new_source, _old_source)),
                        )
                    })
                    .collect();
                let new_else = else_branch
                    .as_ref()
                    .map(|b| Box::new(self.clone_and_update_node(b, new_source, _old_source)));
                Node::new(
                    NodeKind::If {
                        condition: Box::new(new_condition),
                        then_branch: Box::new(new_then),
                        elsif_branches: new_elsif,
                        else_branch: new_else,
                    },
                    SourceLocation { start: new_start, end: new_end },
                )
            }
            NodeKind::Number { .. } => {
                if affected && new_start < new_source.len() && new_end <= new_source.len() {
                    let new_value = &new_source[new_start..new_end];
                    Node::new(
                        NodeKind::Number { value: new_value.to_string() },
                        SourceLocation { start: new_start, end: new_end },
                    )
                } else {
                    self.clone_with_shifted_positions(node, shift)
                }
            }
            NodeKind::String { interpolated, .. } => {
                if affected && new_start < new_source.len() && new_end <= new_source.len() {
                    let new_value = &new_source[new_start..new_end];
                    Node::new(
                        NodeKind::String {
                            value: new_value.to_string(),
                            interpolated: *interpolated,
                        },
                        SourceLocation { start: new_start, end: new_end },
                    )
                } else {
                    self.clone_with_shifted_positions(node, shift)
                }
            }
            _ => self.clone_with_shifted_positions(node, shift),
        }
    }

    fn calculate_shift_at(&self, position: usize) -> isize {
        self.pending_edits.byte_shift_at(position)
    }

    fn calculate_content_delta(&self, node: &Node) -> isize {
        let mut delta = 0;
        let mut cumulative_shift: isize = 0;

        for edit in &self.pending_edits.edits {
            let start = (edit.start_byte as isize - cumulative_shift) as usize;
            let end = (edit.old_end_byte as isize - cumulative_shift) as usize;
            if start >= node.location.start && end <= node.location.end {
                delta += edit.byte_shift();
            }
            cumulative_shift += edit.byte_shift();
        }

        delta
    }

    fn is_node_affected(&self, node: &Node) -> bool {
        let mut cumulative_shift: isize = 0;
        for edit in &self.pending_edits.edits {
            let start = (edit.start_byte as isize - cumulative_shift) as usize;
            let end = (edit.old_end_byte as isize - cumulative_shift) as usize;
            if start < node.location.end && end > node.location.start {
                return true;
            }
            cumulative_shift += edit.byte_shift();
        }
        false
    }

    #[allow(clippy::only_used_in_recursion)]
    fn clone_with_shifted_positions(&self, node: &Node, shift: isize) -> Node {
        let new_location = SourceLocation {
            start: (node.location.start as isize + shift) as usize,
            end: (node.location.end as isize + shift) as usize,
        };

        let new_kind = match &node.kind {
            NodeKind::Program { statements } => NodeKind::Program {
                statements: statements
                    .iter()
                    .map(|s| self.clone_with_shifted_positions(s, shift))
                    .collect(),
            },
            NodeKind::Block { statements } => NodeKind::Block {
                statements: statements
                    .iter()
                    .map(|s| self.clone_with_shifted_positions(s, shift))
                    .collect(),
            },
            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                NodeKind::VariableDeclaration {
                    declarator: declarator.clone(),
                    variable: Box::new(self.clone_with_shifted_positions(variable, shift)),
                    attributes: attributes.clone(),
                    initializer: initializer
                        .as_ref()
                        .map(|i| Box::new(self.clone_with_shifted_positions(i, shift))),
                }
            }
            NodeKind::Binary { op, left, right } => NodeKind::Binary {
                op: op.clone(),
                left: Box::new(self.clone_with_shifted_positions(left, shift)),
                right: Box::new(self.clone_with_shifted_positions(right, shift)),
            },
            NodeKind::Unary { op, operand } => NodeKind::Unary {
                op: op.clone(),
                operand: Box::new(self.clone_with_shifted_positions(operand, shift)),
            },
            NodeKind::FunctionCall { name, args } => NodeKind::FunctionCall {
                name: name.clone(),
                args: args.iter().map(|a| self.clone_with_shifted_positions(a, shift)).collect(),
            },
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => NodeKind::If {
                condition: Box::new(self.clone_with_shifted_positions(condition, shift)),
                then_branch: Box::new(self.clone_with_shifted_positions(then_branch, shift)),
                elsif_branches: elsif_branches
                    .iter()
                    .map(|(c, b)| {
                        (
                            self.clone_with_shifted_positions(c, shift),
                            self.clone_with_shifted_positions(b, shift),
                        )
                    })
                    .map(|(c, b)| (Box::new(c), Box::new(b)))
                    .collect(),
                else_branch: else_branch
                    .as_ref()
                    .map(|b| Box::new(self.clone_with_shifted_positions(b, shift))),
            },
            _ => node.kind.clone(), // For leaf nodes, just clone
        };

        Node::new(new_kind, new_location)
    }

    fn count_reuse_potential(&mut self, old_root: &Node, new_root: &Node) {
        // Compare trees and count which nodes could have been reused
        let (reused, reparsed) = self.analyze_reuse(old_root, new_root);
        self.reused_nodes = reused;
        self.reparsed_nodes = reparsed;
    }

    fn analyze_reuse(&self, old_node: &Node, new_node: &Node) -> (usize, usize) {
        // Check if nodes are structurally equivalent
        match (&old_node.kind, &new_node.kind) {
            (
                NodeKind::Program { statements: old_stmts },
                NodeKind::Program { statements: new_stmts },
            ) => {
                let mut reused = 1; // Program node itself
                let mut reparsed = 0;

                for (old_stmt, new_stmt) in old_stmts.iter().zip(new_stmts.iter()) {
                    let (r, p) = self.analyze_reuse(old_stmt, new_stmt);
                    reused += r;
                    reparsed += p;
                }

                (reused, reparsed)
            }
            (
                NodeKind::VariableDeclaration { variable: old_var, initializer: old_init, .. },
                NodeKind::VariableDeclaration { variable: new_var, initializer: new_init, .. },
            ) => {
                let mut reused = 1; // VarDecl itself
                let mut reparsed = 0;

                let (r, p) = self.analyze_reuse(old_var, new_var);
                reused += r;
                reparsed += p;

                if let (Some(old_i), Some(new_i)) = (old_init, new_init) {
                    let (r, p) = self.analyze_reuse(old_i, new_i);
                    reused += r;
                    reparsed += p;
                }

                (reused, reparsed)
            }
            (NodeKind::Number { value: old_val }, NodeKind::Number { value: new_val }) => {
                if old_val != new_val {
                    (0, 1) // Value changed - reparsed
                } else {
                    (1, 0) // Value same - could have been reused
                }
            }
            (
                NodeKind::Variable { sigil: old_s, name: old_n },
                NodeKind::Variable { sigil: new_s, name: new_n },
            ) => {
                if old_s == new_s && old_n == new_n {
                    (1, 0) // Reused
                } else {
                    (0, 1) // Reparsed
                }
            }
            _ => {
                if self.nodes_match(old_node, new_node) {
                    (1, 0)
                } else {
                    (0, 1)
                }
            }
        }
    }

    fn count_reused(&self, old_node: &Node, new_node: &Node) -> usize {
        // Count nodes that were reused (not reparsed)
        if self.nodes_match(old_node, new_node) {
            1 + self.count_children_reused(old_node, new_node)
        } else {
            self.count_children_reused(old_node, new_node)
        }
    }

    fn nodes_match(&self, node1: &Node, node2: &Node) -> bool {
        match (&node1.kind, &node2.kind) {
            (NodeKind::Number { value: v1 }, NodeKind::Number { value: v2 }) => v1 == v2,
            (NodeKind::String { value: v1, .. }, NodeKind::String { value: v2, .. }) => v1 == v2,
            (
                NodeKind::Variable { sigil: s1, name: n1 },
                NodeKind::Variable { sigil: s2, name: n2 },
            ) => s1 == s2 && n1 == n2,
            _ => true, // Consider structural nodes as reused if their type matches
        }
    }

    fn count_children_reused(&self, old_node: &Node, new_node: &Node) -> usize {
        let mut count = 0;

        match (&old_node.kind, &new_node.kind) {
            (NodeKind::Program { statements: old }, NodeKind::Program { statements: new })
            | (NodeKind::Block { statements: old }, NodeKind::Block { statements: new }) => {
                for (old_stmt, new_stmt) in old.iter().zip(new.iter()) {
                    count += self.count_reused(old_stmt, new_stmt);
                }
            }
            (
                NodeKind::VariableDeclaration { variable: old_v, initializer: old_i, .. },
                NodeKind::VariableDeclaration { variable: new_v, initializer: new_i, .. },
            ) => {
                count += self.count_reused(old_v, new_v);
                if let (Some(old_init), Some(new_init)) = (old_i, new_i) {
                    count += self.count_reused(old_init, new_init);
                }
            }
            _ => {}
        }

        count
    }

    #[allow(clippy::only_used_in_recursion)]
    fn count_nodes(&self, node: &Node) -> usize {
        let mut count = 1;

        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    count += self.count_nodes(stmt);
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                count += self.count_nodes(variable);
                if let Some(init) = initializer {
                    count += self.count_nodes(init);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                count += self.count_nodes(left);
                count += self.count_nodes(right);
            }
            NodeKind::Unary { operand, .. } => {
                count += self.count_nodes(operand);
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    count += self.count_nodes(arg);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                count += self.count_nodes(condition);
                count += self.count_nodes(then_branch);
                for (cond, branch) in elsif_branches {
                    count += self.count_nodes(cond);
                    count += self.count_nodes(branch);
                }
                if let Some(branch) = else_branch {
                    count += self.count_nodes(branch);
                }
            }
            _ => {}
        }

        count
    }
}

impl Default for IncrementalParserV2 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn test_incremental_insertion() {
        let mut parser = IncrementalParserV2::new();

        // Initial parse
        let source1 = "my $x = 42;";
        parser.parse(source1).unwrap();
        // Program + VarDecl + Variable + Number
        assert_eq!(parser.reparsed_nodes, 4);

        // Insert characters into the number
        parser.edit(Edit::new(
            8,
            10,
            12, // "42" -> "4242"
            Position::new(8, 1, 9),
            Position::new(10, 1, 11),
            Position::new(12, 1, 13),
        ));

        let source2 = "my $x = 4242;";
        let tree2 = parser.parse(source2).unwrap();

        assert_eq!(parser.reused_nodes, 3); // Program, VarDecl, Variable
        assert_eq!(parser.reparsed_nodes, 1); // Number

        if let NodeKind::Program { statements } = &tree2.kind {
            if let NodeKind::VariableDeclaration { initializer: Some(init), .. } =
                &statements[0].kind
            {
                if let NodeKind::Number { value } = &init.kind {
                    assert_eq!(value, "4242");
                }
            }
        }
    }

    #[test]
    fn test_incremental_deletion() {
        let mut parser = IncrementalParserV2::new();

        let source1 = "my $x = 4242;";
        parser.parse(source1).unwrap();

        // Delete part of the number
        parser.edit(Edit::new(
            10,
            12,
            10, // remove "42"
            Position::new(10, 1, 11),
            Position::new(12, 1, 13),
            Position::new(10, 1, 11),
        ));

        let source2 = "my $x = 42;";
        parser.parse(source2).unwrap();

        assert_eq!(parser.reused_nodes, 3);
        assert_eq!(parser.reparsed_nodes, 1);
    }

    #[test]
    fn test_multiple_value_edits() {
        let mut parser = IncrementalParserV2::new();

        // Initial parse
        let source1 = "my $x = 10;\nmy $y = 20;";
        parser.parse(source1).unwrap();

        // First edit: insertion
        parser.edit(Edit::new(
            8,
            10,
            11, // "10" -> "100"
            Position::new(8, 1, 9),
            Position::new(10, 1, 11),
            Position::new(11, 1, 12),
        ));

        // Second edit: deletion after adjustment from first edit
        parser.edit(Edit::new(
            22,
            23,
            22, // "20" -> "2"
            Position::new(22, 2, 10),
            Position::new(23, 2, 11),
            Position::new(22, 2, 10),
        ));

        let source2 = "my $x = 100;\nmy $y = 2;";
        parser.parse(source2).unwrap();

        assert_eq!(parser.reused_nodes, 5);
        assert_eq!(parser.reparsed_nodes, 2);
    }
}
