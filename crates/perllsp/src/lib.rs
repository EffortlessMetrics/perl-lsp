//! Public Cargo facade for the `perl-lsp` language server.
//!
//! Install the server with:
//!
//! ```bash
//! cargo install perllsp
//! ```
//!
//! That installs the `perl-lsp` binary while delegating the implementation to
//! the `perl-lsp-rs` crate.

#![deny(unsafe_code)]

pub use perl_lsp::*;
