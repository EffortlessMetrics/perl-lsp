//! LSP diagnostics provider for Perl
//!
//! This crate provides diagnostic generation and linting functionality for Perl code.
//!
//! ## Features
//!
//! - Diagnostic generation from AST
//! - Linting for common mistakes
//! - Deprecated feature detection
//! - Strict warnings
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_diagnostics::DiagnosticsProvider;
//!
//! let provider = DiagnosticsProvider::new();
//! let diagnostics = provider.generate_diagnostics(&ast, source, Some(&workspace_index))?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod dedup;
mod diagnostics;
mod error_nodes;
mod lints;
mod parse_errors;
mod scope;
mod types;
mod walker;

pub use diagnostics::{DiagnosticsProvider, Diagnostic, DiagnosticSeverity};
pub use types::{DiagnosticTag, RelatedInformation};

pub mod lints {
    pub use super::lints::{
        check_common_mistakes,
        check_deprecated_features,
        check_strict_warnings,
    };
}
