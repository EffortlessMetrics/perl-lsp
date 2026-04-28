//! Re-exported configuration microcrate API.
//!
//! The server/workspace configuration subsystem lives in `perl-lsp-config` so
//! it can be reused by integration tooling without pulling in the full server.

pub use perl_lsp_rs_core::config::*;
