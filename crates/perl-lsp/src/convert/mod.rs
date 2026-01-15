//! Type conversions between parser engine types and LSP protocol types.
//!
//! This module provides conversion utilities to translate between the internal parser
//! representation (from `perl-parser`) and the LSP protocol types (from `lsp-types`).
//!
//! # Conversion Categories
//!
//! - **Position & Range** - Converting between byte offsets and LSP Position/Range
//! - **Symbols** - Converting parser symbols to LSP SymbolInformation/DocumentSymbol
//! - **Diagnostics** - Converting parser errors to LSP Diagnostic messages
//! - **Completions** - Converting parser results to LSP CompletionItem
//! - **Locations** - Converting parser locations to LSP Location/LocationLink
//!
//! # UTF-16 Safety
//!
//! LSP uses UTF-16 code units for positions, while Rust strings use UTF-8.
//! All conversions must properly handle multi-byte characters and surrogate pairs.
//!
//! # Wire Types
//!
//! The [`position`] module provides dedicated wire types (`WirePosition`, `WireRange`,
//! `WireLocation`) that are the canonical types for LSP JSON serialization. These types:
//!
//! - Use 0-based line numbers (as required by LSP)
//! - Use UTF-16 code units for character offsets
//! - Convert through byte offsets for correctness
//!
//! Always use wire types when serializing to LSP JSON, not engine types.

pub mod position;

pub use position::{WireLocation, WirePosition, WireRange};
