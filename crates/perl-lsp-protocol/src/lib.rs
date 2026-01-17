//! JSON-RPC protocol types, error handling, and capabilities for perl-lsp.
//!
//! This crate isolates protocol types from the LSP runtime so they can be
//! shared across binaries and provider layers.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod capabilities;
mod errors;
mod jsonrpc;
pub mod methods;

pub use errors::*;
pub use jsonrpc::*;
