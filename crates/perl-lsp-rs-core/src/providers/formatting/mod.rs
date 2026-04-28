//! LSP formatting provider for Perl
//!
//! This crate provides code formatting functionality for Perl using perltidy.
//!
//! ## Features
//!
//! - Perltidy integration
//! - Configurable formatting options
//! - LSP protocol compatibility
//!
//! ## Usage
//!
//! ```rust,ignore
//! use perl_lsp_formatting::{FormattingProvider, PerlTidyConfig};
//! use perl_lsp_tooling::OsSubprocessRuntime;
//!
//! let runtime = OsSubprocessRuntime::new();
//! let config = PerlTidyConfig::default();
//! let provider = FormattingProvider::new(runtime).with_perltidy_config(config);
//! let formatted = provider.format_document(source, &options)?;
//! ```

#[allow(clippy::module_inception)]
mod formatting;

pub use formatting::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingError,
    FormattingOptions, FormattingProvider, PerlTidyConfig,
};
