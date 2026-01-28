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

/// Diagnostic deduplication utilities
mod dedup;
/// Core diagnostics provider
mod diagnostics;
/// ERROR node classification and reporting
mod error_nodes;
/// Lint checks (common mistakes, deprecations, strict warnings)
mod lints;
/// Parse error to diagnostic conversion
mod parse_errors;
/// Scope analysis integration
mod scope;
/// Core diagnostic types
mod types;
/// AST walker utilities
mod walker;

pub use diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider, RelatedInformation,
};

// Re-export lint checks from the lints module
pub use lints::common_mistakes;
pub use lints::deprecated;
pub use lints::strict_warnings;
