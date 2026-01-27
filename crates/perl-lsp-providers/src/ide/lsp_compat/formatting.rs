//! LSP feature module (deprecated)
//!
//! **DEPRECATED**: This module has moved to the `perl-lsp` crate.
//!
//! For backwards compatibility during the migration period, this module
//! is kept as an empty stub. Migrate to `perl_lsp::features::formatting`.
//!
//! # LSP Context
//!
//! Formatting is negotiated via the LSP formatting workflow and honors the
//! server-side formatting capability when enabled.
//!
//! # Client capability requirements
//!
//! Clients must advertise the formatting capability (`textDocument/formatting`
//! or `textDocument/rangeFormatting`) to receive formatting responses.
//!
//! # Protocol compliance
//!
//! The formatting protocol follows LSP request/response semantics and expects
//! edits to be applied atomically by the client.
//!
//! # Migration
//!
//! ```ignore
//! // Old:
//! use perl_parser::formatting;
//!
//! // New:
//! use perl_lsp::features::formatting;
//! ```
