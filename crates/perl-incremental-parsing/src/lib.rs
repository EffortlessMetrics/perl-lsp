//! Incremental parsing compatibility shim for Perl.
//!
//! This crate intentionally re-exports [`perl_parser`] incremental APIs.
//! `perl-parser` is the single source of truth for incremental parsing logic.
//!
//! # Migration
//!
//! Prefer importing incremental APIs directly from `perl_parser` in new code.
//! Existing `perl_incremental_parsing` imports remain supported via re-exports.

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

#[doc(inline)]
pub use perl_parser::edit;
#[doc(inline)]
pub use perl_parser::{Node, NodeKind, Parser, SourceLocation, ast, error, parser, position};

/// Compatibility re-export of incremental parsing APIs from [`perl_parser::incremental`].
#[doc(inline)]
pub use perl_parser::incremental;

#[doc(inline)]
pub use perl_parser::incremental::*;
