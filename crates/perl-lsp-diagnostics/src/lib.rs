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
//! - Security anti-pattern detection
//!
//! ## Usage
//!
//! ```rust,ignore
//! use perl_lsp_diagnostics::DiagnosticsProvider;
//!
//! let provider = DiagnosticsProvider::new();
//! let diagnostics = provider.generate_diagnostics(&ast, source, Some(&workspace_index))?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Dead code detection
#[cfg(not(target_arch = "wasm32"))]
mod dead_code;
/// Diagnostic deduplication utilities
mod dedup;
/// Core diagnostics provider
mod diagnostics;
/// ERROR node classification and reporting
mod error_nodes;
/// Heredoc anti-pattern detection
mod heredoc_antipatterns;
/// Lint checks (common mistakes, deprecations, strict warnings, security)
mod lints;
/// Parse error to diagnostic conversion
mod parse_errors;
/// Scope analysis integration
mod scope;
/// AST walker utilities
mod walker;

pub use diagnostics::DiagnosticsProvider;
pub use heredoc_antipatterns::detect_heredoc_antipatterns;
pub use parse_errors::{parse_error_code, parse_error_severity};
pub use perl_lsp_diagnostic_types::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, RelatedInformation,
};

// Re-export lint checks from the lints module
pub use lints::common_mistakes;
pub use lints::deprecated;
pub use lints::missing_module;
pub use lints::package_subroutine;
/// Same-file Moo/Moose role conflict detection.
pub use lints::role_conflicts;
pub use lints::security;
pub use lints::strict_warnings;
pub use lints::unreachable_code;
pub use lints::unused_imports;
pub use lints::version_compat;

// Re-export dead code detection (when not targeting WASM)
#[cfg(not(target_arch = "wasm32"))]
pub use dead_code::detect_dead_code;
