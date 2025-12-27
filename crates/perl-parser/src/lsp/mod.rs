//! LSP Server implementation modules
//!
//! This module provides a modular Language Server Protocol implementation
//! organized into coherent subsystems:
//!
//! - **protocol**: JSON-RPC message types and error handling
//! - **transport**: Message framing and I/O
//! - **state**: Document and server state management
//! - **dispatch**: Request routing and lifecycle management
//! - **handlers**: LSP method implementations
//! - **fallback**: Text-based fallback implementations

pub mod dispatch;
pub mod fallback;
pub mod handlers;
pub mod protocol;
pub mod server;
pub mod state;
pub mod transport;

// Re-export primary types for backward compatibility
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use server::LspServer;
