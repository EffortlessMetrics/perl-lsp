//! LSP URI compatibility module.
//!
//! URI parsing helpers now live in the `perl-lsp-uri` microcrate.
//! This module re-exports the public API for compatibility.

// Wave G3 (#4535): perl-lsp-uri absorbed into perl-lsp-rs-core::uri
pub use crate::uri::parse_uri;
