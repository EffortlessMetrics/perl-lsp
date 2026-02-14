//! Tooling integration for Perl LSP
//!
//! This crate provides abstractions for integrating with external Perl tooling
//! such as perltidy (formatting) and perlcritic (linting).
//!
//! ## Features
//!
//! - Subprocess execution abstraction
//! - Mock implementations for testing
//! - WASM compatibility
//!
//! ## Usage
//!
//! ```rust
//! use perl_lsp_tooling::{SubprocessRuntime, OsSubprocessRuntime};
//!
//! let runtime = OsSubprocessRuntime::new();
//! let output = runtime.run_command("perltidy", &["-st"], Some(b"code"))?;
//! ```

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::empty_line_after_outer_attr)]

/// Performance optimizations for large projects.
pub mod performance;
/// Perl::Critic integration for code quality analysis.
pub mod perl_critic;
/// Perltidy integration for code formatting.
pub mod perltidy;
mod subprocess_runtime;

pub use subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};

#[cfg(not(target_arch = "wasm32"))]
pub use subprocess_runtime::OsSubprocessRuntime;

/// Test mock implementations for subprocess runtimes.
#[cfg(test)]
pub mod mock {
    pub use crate::subprocess_runtime::mock::*;
}
