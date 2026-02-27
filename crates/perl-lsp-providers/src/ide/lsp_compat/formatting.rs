//! LSP formatting provider (deprecated)
//!
//! **DEPRECATED**: This module has moved to `perl-lsp-formatting` crate.
//!
//! For backwards compatibility during the migration period, this module
//! re-exports types from `perl-lsp-formatting`.
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
//! use perl_lsp_providers::formatting::{FormattingProvider, FormattingOptions};
//!
//! // New:
//! use perl_lsp_formatting::{FormattingProvider, FormattingOptions};
//! ```

// Re-export all public types from perl-lsp-formatting for backward compatibility
pub use perl_lsp_formatting::*;
