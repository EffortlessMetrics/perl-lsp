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

// TODO: Implement conversion functions
// - Engine Position/Range → lsp_types::Position/Range
// - Engine Symbol → lsp_types::SymbolInformation
// - Engine Error → lsp_types::Diagnostic
// - Engine Location → lsp_types::Location
