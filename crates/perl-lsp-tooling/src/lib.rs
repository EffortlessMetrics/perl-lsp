//! Tooling integration for Perl LSP
//!
//! This crate provides reusable integrations for external Perl tooling and
//! performance infrastructure. Perltidy-specific formatting types now live in
//! the dedicated `perl-lsp-perltidy` microcrate and are re-exported here for
//! compatibility.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::empty_line_after_outer_attr)]

/// Performance optimizations for large projects.
pub mod performance {
    pub use perl_lsp_performance::*;
}
/// Perl::Critic integration for code quality analysis.
pub mod perl_critic;
/// Perltidy integration for code formatting.
pub mod perltidy {
    pub use perl_lsp_perltidy::*;
}

pub use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};

#[cfg(not(target_arch = "wasm32"))]
pub use perl_subprocess_runtime::OsSubprocessRuntime;

/// Test mock implementations for subprocess runtimes.
pub mod mock {
    pub use perl_subprocess_runtime::mock::*;
}
