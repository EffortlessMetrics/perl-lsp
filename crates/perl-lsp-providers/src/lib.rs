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

/// Re-exports from `perl_lsp_tooling` for backward compatibility.
pub mod tooling {
    pub use perl_lsp_tooling::*;
}

/// Deprecated alias for [`tooling`].
///
/// Use `perl_lsp_providers::tooling` instead.
#[deprecated(since = "0.9.0", note = "Use perl_lsp_providers::tooling instead")]
pub mod tooling_export {
    pub use super::tooling::*;
}

/// Re-exports from `perl_lsp_diagnostics` for backward compatibility.
pub mod diagnostics {
    pub use perl_lsp_diagnostics::*;
}

/// Re-exports from `perl_lsp_formatting` for backward compatibility.
pub mod formatting {
    pub use perl_lsp_formatting::*;
}

/// Re-exports from `perl_lsp_semantic_tokens` for backward compatibility.
pub mod semantic_tokens {
    pub use perl_lsp_semantic_tokens::*;
}

/// Re-exports from `perl_lsp_inlay_hints` for backward compatibility.
pub mod inlay_hints {
    pub use perl_lsp_inlay_hints::*;
}

/// Re-exports from `perl_lsp_rename` for backward compatibility.
pub mod rename {
    pub use perl_lsp_rename::*;
}

/// Re-exports from `perl_lsp_completion` for backward compatibility.
pub mod completion {
    pub use perl_lsp_completion::*;
}

/// Re-exports from `perl_lsp_code_actions` for backward compatibility.
pub mod code_actions {
    pub use perl_lsp_code_actions::*;
}

/// Re-exports from `perl_lsp_navigation` for backward compatibility.
pub mod navigation {
    pub use perl_lsp_navigation::*;
}
