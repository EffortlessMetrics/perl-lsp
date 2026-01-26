//! Perl AST Library
//!
//! Provides the Abstract Syntax Tree definitions for Perl.
//!
//! - `ast`: The primary (v1) AST used by the current parser.
//! - `v2`: The experimental (v2) AST with incremental parsing support.

pub mod ast;
pub mod v2;

pub use ast::{Node, NodeKind};
pub use perl_position_tracking::SourceLocation;
