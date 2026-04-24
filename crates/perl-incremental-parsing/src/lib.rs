//! Compatibility shim for incremental parsing APIs.
//!
//! `perl-parser` is the single source of truth for incremental parsing.
//! This crate remains as a thin wrapper so existing imports continue to compile.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Compatibility alias to incremental parsing APIs owned by `perl-parser`.
#[deprecated(
    note = "Incremental parsing is owned by `perl-parser`; depend on `perl-parser` directly when possible"
)]
pub use perl_parser::incremental;

pub use perl_parser::edit;
pub use perl_parser::{Node, NodeKind, SourceLocation};
pub use perl_parser::{Parser, ast, error, parser, position};
