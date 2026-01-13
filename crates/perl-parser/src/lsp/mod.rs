//! LSP Server implementation modules
//!
//! This module provides a modular Language Server Protocol implementation
//! organized into coherent subsystems:
//!
//! - **protocol**: JSON-RPC message types and error handling
//! - **transport**: Message framing and I/O
//! - **state**: Document and server state management
//! - **dispatch**: Placeholder (real dispatch in `server_impl/dispatch.rs`)
//! - **handlers**: LSP method implementations
//! - **features**: LSP feature providers (completion, code actions, etc.)
//! - **fallback**: Text-based fallback implementations
//! - **server_impl**: Core LspServer implementation and dispatch logic
//! - **server**: Public server interface (re-exports from server_impl)

pub mod dispatch;
pub mod fallback;
pub mod features;
pub mod handlers;
pub mod protocol;
pub mod state;
pub mod utils;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
pub mod server_impl;
#[cfg(not(target_arch = "wasm32"))]
pub mod transport;

// Re-export primary types for backward compatibility
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
#[cfg(not(target_arch = "wasm32"))]
pub use server::LspServer;
