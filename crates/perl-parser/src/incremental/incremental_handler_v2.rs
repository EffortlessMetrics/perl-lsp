//! Incremental LSP didChange handler (deprecated).
//!
//! **DEPRECATED**: The LSP server implementation has moved to the `perl-lsp` crate.
//! This module is kept as a stub for compatibility and no longer provides
//! incremental didChange handling in `perl-parser`.
//!
//! # Migration
//!
//! ```ignore
//! // Old:
//! use perl_parser::incremental_handler_v2;
//!
//! // New:
//! use perl_lsp::server;
//! ```
