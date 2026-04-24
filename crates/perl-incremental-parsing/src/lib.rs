//! Compatibility shim for incremental parsing APIs.
//!
//! Source of truth for incremental parsing now lives in `perl-parser` under
//! [`perl_parser::incremental`]. This crate remains as a thin re-export so
//! downstream callers can migrate without behavior changes.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub use perl_parser::edit;
pub use perl_parser::{Node, NodeKind, SourceLocation};
pub use perl_parser::{Parser, ast, error, parser, position};

/// Incremental parsing implementation and helpers from `perl-parser`.
#[deprecated(note = "use `perl_parser::incremental` directly; this crate is a compatibility shim")]
pub use perl_parser::incremental;

pub use perl_parser::incremental::*;
