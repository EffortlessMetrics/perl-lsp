//! LSP provider shims and tooling integrations for Perl.
//!
//! This crate provides Language Server Protocol feature implementations and
//! integrations with external Perl tooling (perltidy, perlcritic) for the
//! Perl LSP ecosystem.
//!
//! # Overview
//!
//! The providers crate offers:
//! - IDE integration shims for LSP/DAP runtime support
//! - Tooling integrations for formatting (perltidy) and linting (perlcritic)
//! - Performance utilities for LSP feature optimization
//!
//! # Example
//!
//! Provider usage depends on specific IDE and tooling module implementations.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::wildcard_imports,
    clippy::enum_glob_use,
    clippy::match_same_arms,
    clippy::if_not_else,
    clippy::struct_excessive_bools,
    clippy::items_after_statements,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::collapsible_match,
    clippy::collapsible_if,
    clippy::only_used_in_recursion,
    clippy::items_after_test_module,
    clippy::while_let_loop,
    clippy::single_range_in_vec_init,
    clippy::arc_with_non_send_sync,
    clippy::needless_range_loop,
    clippy::result_large_err,
    clippy::if_same_then_else,
    clippy::should_implement_trait,
    clippy::manual_flatten,
    clippy::needless_raw_string_hashes,
    clippy::single_char_pattern,
    clippy::uninlined_format_args
)]

pub use perl_parser_core::{Node, NodeKind, SourceLocation};
pub use perl_parser_core::{Parser, ast, position};

/// IDE integration helpers (LSP/DAP runtime support).
pub mod ide;
/// Tooling integrations and performance helpers.
pub mod tooling;
