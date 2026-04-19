//! LSP URI compatibility module.
//!
//! URI parsing helpers now live in the `perl-lsp-uri` microcrate.
//! This module re-exports the public API for compatibility.

pub use perl_lsp_uri::parse_uri;
