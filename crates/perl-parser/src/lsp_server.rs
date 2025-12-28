//! LSP Server compatibility shim
//!
//! This module re-exports from the new modular location at `crate::lsp::server`.
//! The actual implementation lives in `lsp/server_impl.rs`.
//!
//! ## Migration Note
//! External code should migrate to using `perl_parser::lsp::server::LspServer`
//! or the crate root re-export `perl_parser::LspServer` (for v0.9+).
//!
//! This shim will be deprecated in a future release.

// Re-export LspServer from the new canonical location
pub use crate::lsp::server::LspServer;

// Re-export protocol types for backward compatibility
pub use crate::lsp::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
