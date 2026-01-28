//! AST utilities for code actions
//!
//! Provides helper functions for walking and analyzing the AST to find
//! nodes and positions for code actions.

use perl_parser_core::{Node, NodeKind};

/// Find the best position to insert a declaration
pub fn find_declaration_position(source: &str, error_pos: usize) -> usize {
    // Find the start of the current statement
    find_statement_start(source, error_pos)
}

/// Find the start of the current statement
pub fn find_statement_start(source: &str, pos: usize) -> usize {
    // Look backwards for statement boundary
    let mut i = pos.saturating_sub(1);
    while i > 0 {
        if source.chars().nth(i) == Some(';') || source.chars().nth(i) == Some('\n') {
            return i + 1;
        }
        i = i.saturating_sub(1);
    }
    0
}

/// Find a good position to insert a function
pub fn find_function_insert_position(source: &str) -> usize {
    // For now, insert at the end of the file
    source.len()
}

/// Find node at the given range
#[allow(clippy::only_used_in_recursion)]
pub fn find_node_at_range(node: &Node, range: (usize, usize)) -> Option<&Node> {
    // Check if this node contains the range
    if node.location.start <= range.0 && node.location.end >= range.1 {
        // Check children for more specific match based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    if let Some(result) = find_node_at_range(stmt, range) {
                        return Some(result);
                    }
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    if let Some(result) = find_node_at_range(stmt, range) {
                        return Some(result);
                    }
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                if let Some(result) = find_node_at_range(condition, range) {
                    return Some(result);
                }
                if let Some(result) = find_node_at_range(then_branch, range) {
                    return Some(result);
                }
                for (cond, branch) in elsif_branches {
                    if let Some(result) = find_node_at_range(cond, range) {
                        return Some(result);
                    }
                    if let Some(result) = find_node_at_range(branch, range) {
                        return Some(result);
                    }
                }
                if let Some(branch) = else_branch {
                    if let Some(result) = find_node_at_range(branch, range) {
                        return Some(result);
                    }
                }
            }
            NodeKind::Binary { left, right, .. } => {
                if let Some(result) = find_node_at_range(left, range) {
                    return Some(result);
                }
                if let Some(result) = find_node_at_range(right, range) {
                    return Some(result);
                }
            }
            _ => {}
        }
        return Some(node);
    }
    None
}

/// Get indentation at a position
pub fn get_indent_at(source: &str, pos: usize) -> String {
    let line_start = source[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);

    let line = &source[line_start..];
    let mut indent = String::new();
    for ch in line.chars() {
        if ch == ' ' || ch == '\t' {
            indent.push(ch);
        } else {
            break;
        }
    }
    indent
}
