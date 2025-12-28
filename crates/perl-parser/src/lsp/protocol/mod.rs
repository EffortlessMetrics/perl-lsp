//! JSON-RPC protocol types and error handling
//!
//! This module contains the core JSON-RPC 2.0 message types used for
//! LSP communication, along with standardized error codes and response builders.

mod errors;
mod jsonrpc;

pub use errors::*;
pub use jsonrpc::*;
