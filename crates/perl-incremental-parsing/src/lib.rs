//! Compatibility wrapper for incremental parsing APIs.
//!
//! `perl-parser` is the canonical owner of incremental parsing logic.
//! This crate is kept as a thin compatibility shim that re-exports those
//! APIs to avoid downstream breakage during migration.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub use perl_parser::ast;
pub use perl_parser::edit;
pub use perl_parser::error;
pub use perl_parser::parser;
pub use perl_parser::position;
pub use perl_parser::{Node, NodeKind, Parser, SourceLocation};

/// Incremental parsing implementation and helpers (re-exported from `perl-parser`).
pub mod incremental;
pub use incremental::*;
