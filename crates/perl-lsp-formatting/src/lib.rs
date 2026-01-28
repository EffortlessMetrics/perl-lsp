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
//! ```rust
//! use perl_lsp_formatting::FormattingProvider;
//! use perl_lsp_tooling::OsSubprocessRuntime;
//!
//! let runtime = OsSubprocessRuntime::new();
//! let provider = FormattingProvider::new(runtime);
//! let formatted = provider.format_document(source, &options)?;
//! ```

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

mod formatting;

pub use formatting::{
    FormatPosition, FormatRange, FormatTextEdit, FormattedDocument, FormattingError,
    FormattingOptions, FormattingProvider,
};
