//! Re-exported cancellation microcrate API.
//!
//! The cancellation subsystem now lives in `perl-lsp-cancellation` and is reused by
//! both LSP internals and external integrations that need explicit API access.

pub use perl_lsp_rs_core::runtime::cancellation::*;
