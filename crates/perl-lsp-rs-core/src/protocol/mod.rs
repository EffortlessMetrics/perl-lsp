//! JSON-RPC protocol types, error handling, and capabilities for perl-lsp.
//!
//! This module isolates protocol types from the LSP runtime so they can be
//! shared across binaries and provider layers. Key submodules:
//!
//! - `jsonrpc` — Core JSON-RPC 2.0 request, response, and error message types
//! - `errors` — Standard and LSP-specific JSON-RPC error codes and builders
//! - [`methods`] — LSP 3.17 method name constants for request/notification routing
//! - [`capabilities`] — Server capability configuration advertised during `initialize`
//!
//! Previously the standalone `perl-lsp-protocol` crate; absorbed into
//! `perl-lsp-rs-core::protocol` in Wave G3 (#4535).

pub mod capabilities;
mod errors;
mod jsonrpc;
pub mod methods;

pub use errors::*;
pub use jsonrpc::*;

/// Convenience function: create an LSP error (ServerErrorStart code).
pub fn lsp_error(message: &str) -> JsonRpcError {
    JsonRpcError::new(crate::protocol::SERVER_ERROR_START, message)
}
