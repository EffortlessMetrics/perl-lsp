//! Test-framework awareness for LSP providers.
//!
//! This module hosts framework-specific fact tables and discovery logic that
//! teach LSP providers (completion, hover, code lens, document symbols,
//! diagnostics/critic) about Perl testing frameworks. It is deliberately
//! LSP-facing but subprocess-free: actual test *execution* lives in the
//! `perl-lsp-rs` runtime (`execute_command`), not here.
//!
//! Today it covers [Test2](https://metacpan.org/pod/Test2::V0) via [`test2`].

pub mod subtest;
pub mod tap;
pub mod test2;
