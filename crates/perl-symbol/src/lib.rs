//! Unified Perl symbol model: taxonomy, cursor extraction, search indexing, and AST projection.
//!
//! This crate consolidates the former `perl-symbol-types`, `perl-symbol-cursor`,
//! `perl-symbol-index`, and `perl-symbol-surface` microcrates into a single published
//! facade (see ADR-0041). Internal module boundaries are preserved; the public surface
//! is defined by `api.rs`.
//!
//! # Modules
//!
//! - [`types`] — Symbol taxonomy: `SymbolKind`, `VarKind`, and LSP mappings
//! - [`cursor`] — Cursor-based symbol extraction helpers
//! - [`index`] — Trie + inverted-index symbol search
//! - [`surface`] — Projection layer: derives symbol views from Perl AST
//!
//! # Quick start
//!
//! ```rust,ignore
//! use perl_symbol::{SymbolKind, VarKind};
//!
//! let var = SymbolKind::Variable(VarKind::Scalar);
//! assert_eq!(var.sigil(), Some("$"));
//! ```

#![warn(missing_docs)]

pub mod cursor;
pub mod index;
pub mod surface;
pub mod types;

pub mod api;
pub use api::*;
