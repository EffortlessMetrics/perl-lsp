//! LSP Server core
//!
//! This module provides the public interface to the LSP server implementation.
//! The actual implementation lives in `server_impl.rs`.
//!
//! ## Migration Status
//! The server implementation has been moved from the legacy `lsp_server.rs`
//! (at the crate root) to `lsp/server_impl.rs`. The root-level `lsp_server.rs`
//! now serves as a compatibility shim that re-exports from here.

// Re-export LspServer from the implementation module
pub use super::server_impl::LspServer;

// Re-export protocol types for convenience
pub use super::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
