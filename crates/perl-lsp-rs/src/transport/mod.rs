//! LSP transport layer
//!
//! Handles message framing with Content-Length headers according to
//! the LSP Base Protocol specification.
//!
//! Wave G3 (#4535): `perl-lsp-transport` absorbed into `perl-lsp-rs-core::transport`.
//! This module re-exports the transport layer for backward compatibility.

pub use perl_lsp_rs_core::transport::*;
