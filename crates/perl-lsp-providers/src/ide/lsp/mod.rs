//! LSP compatibility module (deprecated)
//!
//! **NOTE**: The LSP implementation has moved to the `perl-lsp` crate.
//! This module exists only for backwards compatibility and will be removed
//! in a future release.
//!
//! ## Migration
//!
//! If you were using `perl_parser::lsp::*`, migrate to `perl_lsp::*`:
//!
//! ```ignore
//! // Old (deprecated):
//! use perl_parser::lsp::LspServer;
//!
//! // New:
//! use perl_lsp::LspServer;
//! ```
//!
//! The `perl-parser` crate is now the engine-only library for parsing Perl code.
//! The `perl-lsp` crate is the LSP runtime that uses `perl-parser` as its engine.

// This module is intentionally empty.
// The LSP implementation has been moved to the perl-lsp crate.
//
// For backwards compatibility during the migration period, this module
// is kept as a stub behind the `lsp-compat` feature flag.
