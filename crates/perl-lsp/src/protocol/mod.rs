//! JSON-RPC protocol types, error handling, and capabilities.
//!
//! This module re-exports the protocol layer from the `perl-lsp-protocol` crate.

pub use perl_lsp_protocol::*;

pub use perl_lsp_request_params::{req_position, req_range, req_uri};
