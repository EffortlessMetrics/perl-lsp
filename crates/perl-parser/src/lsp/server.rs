//! LSP Server core
//!
//! This module re-exports the LspServer from the legacy lsp_server module
//! during the migration period. Eventually, the server implementation will
//! live here directly.

// Re-export LspServer from legacy location
pub use crate::lsp_server::LspServer;

// Re-export protocol types from the new modular location
pub use super::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

// After migration is complete, this file will contain:
// - LspServer struct definition
// - Constructor methods (new, with_output)
// - run() event loop
// - Core server state
