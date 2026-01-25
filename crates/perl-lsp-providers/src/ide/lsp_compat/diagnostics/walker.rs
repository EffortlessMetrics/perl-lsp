//! AST walker for diagnostics
//!
//! This module provides a generic AST walker function for traversing
//! Perl AST nodes and applying diagnostic checks.

use perl_parser_core::ast::{Node, NodeKind};

/// Walk the AST and call a function for each node
///
/// This function recursively walks the AST and calls the provided function
/// for each node. The function is called before visiting children (pre-order).
#[allow(clippy::only_used_in_recursion)]
pub fn walk_node<F>(node: &Node, func: &mut F)
where
    F: FnMut(&Node),
{
    func(node);

    // Visit children based on node kind
    match &node.kind {
        NodeKind::Program { statements } => {
            for stmt in statements {
                walk_node(stmt, func);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                walk_node(stmt, func);
            }
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            walk_node(condition, func);
            walk_node(then_branch, func);
            for (cond, branch) in elsif_branches {
                walk_node(cond, func);
                walk_node(branch, func);
            }
            if let Some(branch) = else_branch {
                walk_node(branch, func);
            }
        }
        NodeKind::While { condition, body, .. } => {
            walk_node(condition, func);
            walk_node(body, func);
        }
        NodeKind::Binary { left, right, .. } => {
            walk_node(left, func);
            walk_node(right, func);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                walk_node(arg, func);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            walk_node(expression, func);
        }
        _ => {} // Other nodes don't have children or are handled differently
    }
}
