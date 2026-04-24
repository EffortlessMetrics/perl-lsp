//! Incremental parsing compatibility crate.
//!
//! The canonical incremental parsing implementation now lives in `perl-parser`.
//! This crate is retained as a thin shim so existing downstream imports keep
//! working while migration to `perl-parser::incremental` is completed.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub use perl_parser::{Node, NodeKind, Parser, SourceLocation, ast, edit, error, parser, position};

/// Legacy incremental parsing namespace.
#[deprecated(note = "Use `perl_parser::incremental` and related `perl_parser::*` re-exports")]
pub mod incremental;

#[allow(deprecated)]
pub use incremental::*;
