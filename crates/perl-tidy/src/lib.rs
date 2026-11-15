//! Rust-native, parse-gated Perl source formatting.
//!
//! `perl-tidy` owns formatter policy, layout, diagnostics, and edit
//! production. It deliberately knows nothing about LSP transport,
//! UTF-16 protocol coordinates beyond its current compatibility API,
//! subprocess execution, or filesystem replacement.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

/// Native formatter implementation and public contracts.
pub mod native;

pub use native::{
    BracePlacement, ElsePlacement, FinalNewline, FormatConfig, FormatDiagnostic,
    FormatDiagnosticSeverity, FormatDoc, FormatResult, FormatterMode, KeywordSpacing,
    NativeFormatter, PerlFormatter, TextEdit, TextPosition, TextRange, TrailingComma,
};
