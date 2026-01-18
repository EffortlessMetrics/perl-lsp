//! LSP transport layer
//!
//! Handles message framing with Content-Length headers according to
//! the LSP Base Protocol specification.
//!
//! This module re-exports the transport layer from the `perl-lsp-transport` crate
//! for backward compatibility.

pub use perl_lsp_transport::*;
