//! AST facade for the core parser engine.
//!
//! This module re-exports AST node definitions from `perl-ast` and anchors them
//! in the parser engine for the Parse → Index → Navigate → Complete → Analyze
//! workflow used by LSP providers and workspace tooling.
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use perl_parser_core::engine::ast::{Node, NodeKind};
//! use perl_parser_core::SourceLocation;
//!
//! let node = Node::new(NodeKind::Empty, SourceLocation { start: 0, end: 0 });
//! assert!(matches!(node.kind, NodeKind::Empty));
//! ```

/// Re-exported AST node types used during Parse/Index/Analyze stages.
pub use perl_ast::ast::*;

/// Walk an AST in pre-order and invoke `visitor` for each node.
///
/// Returns early when `visitor` returns `false`.
pub fn walk_preorder_while<F>(node: &Node, visitor: &mut F) -> bool
where
    F: FnMut(&Node) -> bool,
{
    if !visitor(node) {
        return false;
    }

    let mut keep_going = true;
    node.for_each_child(|child| {
        if keep_going && !walk_preorder_while(child, visitor) {
            keep_going = false;
        }
    });

    keep_going
}

/// Walk an AST in pre-order and invoke `visitor` for each node.
pub fn walk_preorder<F>(node: &Node, visitor: &mut F)
where
    F: FnMut(&Node),
{
    let mut wrapper = |current: &Node| {
        visitor(current);
        true
    };
    let _ = walk_preorder_while(node, &mut wrapper);
}
