//! Compatibility shim for incremental parsing.
//!
//! Incremental parsing is owned by `perl-parser` at [`perl_parser::incremental`].
//! This crate is kept as a thin wrapper to preserve existing imports while making
//! a single source of truth explicit.
//!
//! New code should prefer depending directly on `perl-parser`.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub use perl_parser::edit;
pub use perl_parser::{Node, NodeKind, SourceLocation};
pub use perl_parser::{Parser, ast, error, parser, position};

/// Incremental parsing implementation and helpers from `perl-parser`.
pub use perl_parser::incremental;

/// Back-compat re-exports from [`incremental`].
pub use perl_parser::incremental::*;
