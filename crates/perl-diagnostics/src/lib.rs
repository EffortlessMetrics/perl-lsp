#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

//! Unified diagnostic codes, types, and catalog for Perl LSP.
//!
//! This crate consolidates three previously separate diagnostic crates:
//! - `perl-diagnostics-codes` — stable diagnostic codes, severity, and tags (now `codes` module)
//! - `perl-lsp-diagnostic-types` — diagnostic model types (Diagnostic, RelatedInformation) (now `types` module)
//! - `perl-lsp-diagnostic-catalog` — LSP metadata builders for codes (now `catalog` module)
//!
//! # Modules
//!
//! - [`codes`] — canonical `DiagnosticCode`, `DiagnosticCategory`, `DiagnosticSeverity`, `DiagnosticTag`
//! - [`types`] — `Diagnostic` and `RelatedInformation` structs; `DiagnosticSeverity` and `DiagnosticTag` are re-exported from [`codes`]
//! - [`catalog`] — LSP metadata catalog functions
//!
//! # Type unification
//!
//! `DiagnosticSeverity` and `DiagnosticTag` are single canonical types defined in [`codes`].
//! The [`types`] module re-exports them so the legacy `types::DiagnosticSeverity` import
//! path still resolves to the same underlying type.
//!
//! # Re-exports
//!
//! The crate root re-exports all public items via [`api`].

pub mod catalog;
pub mod codes;
pub mod types;

mod api;
pub use api::*;
