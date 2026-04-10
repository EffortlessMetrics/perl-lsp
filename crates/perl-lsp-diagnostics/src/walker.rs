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
        NodeKind::IndirectCall { object, args, .. } => {
            walk_node(object, func);
            for arg in args {
                walk_node(arg, func);
            }
        }
        NodeKind::ExpressionStatement { expression } => {
            walk_node(expression, func);
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            walk_node(lhs, func);
            walk_node(rhs, func);
        }
        NodeKind::VariableDeclaration { initializer: Some(init), .. } => {
            walk_node(init, func);
        }
        NodeKind::VariableListDeclaration { initializer: Some(init), .. } => {
            walk_node(init, func);
        }
        NodeKind::PhaseBlock { block, .. } => {
            walk_node(block, func);
        }
        NodeKind::Eval { block } => {
            walk_node(block, func);
        }
        NodeKind::Class { body, .. } => {
            walk_node(body, func);
        }
        NodeKind::Try { body, catch_blocks, finally_block } => {
            walk_node(body, func);
            for (_, catch_body) in catch_blocks {
                walk_node(catch_body, func);
            }
            if let Some(finally) = finally_block {
                walk_node(finally, func);
            }
        }
        NodeKind::Given { expr, body } => {
            walk_node(expr, func);
            walk_node(body, func);
        }
        NodeKind::When { condition, body } => {
            walk_node(condition, func);
            walk_node(body, func);
        }
        NodeKind::Default { body } => {
            walk_node(body, func);
        }
        NodeKind::Unary { operand, .. } => {
            walk_node(operand, func);
        }
        NodeKind::Package { block: Some(blk), .. } => {
            walk_node(blk, func);
        }
        NodeKind::Subroutine { body, .. } | NodeKind::Method { body, .. } => {
            walk_node(body, func);
        }
        NodeKind::For { body, .. } | NodeKind::Foreach { body, .. } => {
            walk_node(body, func);
        }
        NodeKind::MethodCall { object, args, .. } => {
            walk_node(object, func);
            for arg in args {
                walk_node(arg, func);
            }
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            walk_node(condition, func);
            walk_node(then_expr, func);
            walk_node(else_expr, func);
        }
        NodeKind::Return { value: Some(v) } => {
            walk_node(v, func);
        }
        NodeKind::LabeledStatement { statement, .. } => {
            walk_node(statement, func);
        }
        NodeKind::HashLiteral { pairs } => {
            for (key, value) in pairs {
                walk_node(key, func);
                walk_node(value, func);
            }
        }
        NodeKind::ArrayLiteral { elements } => {
            for elem in elements {
                walk_node(elem, func);
            }
        }
        _ => {} // Other nodes don't have children or are handled differently
    }
}
