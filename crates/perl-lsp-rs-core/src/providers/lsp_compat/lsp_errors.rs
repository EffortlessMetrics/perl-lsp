//! LSP error types for compatibility providers.
//!
//! This module centralizes error metadata used by the LSP compatibility shims.
//! Errors documented here describe when they occur in the LSP workflow and how
//! clients can recover or retry.
//!
//! # LSP Workflow Integration
//!
//! - **Parse**: Errors surface when invalid documents or positions are received
//! - **Index**: Errors report missing workspace state during symbol lookups
//! - **Navigate**: Errors communicate unresolved definitions or references
//! - **Complete**: Errors indicate unsupported completion requests or bad params
//! - **Analyze**: Errors describe diagnostics failures and recovery strategies
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_lsp_providers::ide::lsp_compat::lsp_errors::LspError;
//!
//! let err = LspError {
//!     code: -32602,
//!     message: "Invalid params".to_string(),
//! };
//! assert_eq!(err.code, -32602);
//! ```

#[derive(Debug, Clone, PartialEq, Eq)]
/// LSP error metadata for compatibility responses.
///
/// Use this type when an LSP workflow stage needs to surface a structured error
/// to the client. Documented error metadata enables consistent recovery messaging
/// across Parse, Index, Navigate, Complete, and Analyze stages, and maps to the
/// Extract → Normalize → Thread → Render → Index pipeline for error reporting.
/// Recovery guidance should explain when the error occurs and what clients can retry.
pub struct LspError {
    /// JSON-RPC error code returned to the client.
    pub code: i32,
    /// Human-readable message describing when this error occurs.
    pub message: String,
}
