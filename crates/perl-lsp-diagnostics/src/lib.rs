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
//! let provider = DiagnosticsProvider::new(&ast, source.to_string());
//! let diagnostics = provider.get_diagnostics(&ast, &parse_errors, source);
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod diagnostics;

pub use diagnostics::{DiagnosticsProvider, Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation};

pub mod lints {
    pub use super::diagnostics::lints::{
        check_common_mistakes,
        check_deprecated_syntax,
        check_strict_warnings,
    };
}
