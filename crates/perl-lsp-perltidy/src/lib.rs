//! Compatibility facade for the formatter crate split.
//!
//! New native consumers should depend on `perl-tidy`; consumers that
//! explicitly invoke external `perltidy` should depend on
//! `perl-tidy-perltidy`. This crate preserves the historical public paths
//! while downstream callers migrate.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

/// Historical native-module path, re-exported from `perl-tidy`.
pub mod native {
    pub use perl_tidy::native::*;
}

pub use perl_tidy::{
    BracePlacement, ElsePlacement, FinalNewline, FormatConfig, FormatDiagnostic,
    FormatDiagnosticSeverity, FormatDoc, FormatResult, FormatterMode, KeywordSpacing,
    NativeFormatter, PerlFormatter, TextEdit, TextPosition, TextRange, TrailingComma,
};
pub use perl_tidy_perltidy::{
    BuiltInFormatter, FormatSuggestion, PerlTidyConfig, PerlTidyFormatter,
};
