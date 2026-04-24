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

/// Walk an AST in pre-order (node before descendants).
///
/// This traversal delegates child discovery to [`Node::for_each_child`], which is
/// the canonical source of truth for child-bearing [`NodeKind`] variants.
pub fn walk_preorder<F>(node: &Node, f: &mut F)
where
    F: FnMut(&Node),
{
    f(node);
    node.for_each_child(|child| walk_preorder(child, f));
}
