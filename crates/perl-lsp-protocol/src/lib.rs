//! JSON-RPC protocol types, error handling, and capabilities for perl-lsp.
//!
//! This crate isolates protocol types from the LSP runtime so they can be
//! shared across binaries and provider layers. Key modules:
//!
//! - `jsonrpc` — Core JSON-RPC 2.0 request, response, and error message types
//! - `errors` — Standard and LSP-specific JSON-RPC error codes and builders
//! - [`methods`] — LSP 3.17 method name constants for request/notification routing
//! - [`capabilities`] — Server capability configuration advertised during `initialize`

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod capabilities;
mod errors;
mod jsonrpc;
pub mod methods;

pub use errors::*;
pub use jsonrpc::*;
