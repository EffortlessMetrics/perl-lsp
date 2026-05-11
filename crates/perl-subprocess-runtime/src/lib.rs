//! Subprocess execution abstraction for provider purity
//!
//! This crate provides a trait-based abstraction for subprocess execution,
//! enabling testing with mock implementations and WASM compatibility.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::panic, clippy::unwrap_used, clippy::expect_used))]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

mod error;
#[cfg(not(target_arch = "wasm32"))]
mod os_runtime;
mod output;

/// Mock subprocess runtime implementations for tests.
pub mod mock;

pub use error::SubprocessError;
#[cfg(not(target_arch = "wasm32"))]
pub use os_runtime::OsSubprocessRuntime;
pub use output::SubprocessOutput;

/// Abstraction trait for subprocess execution.
pub trait SubprocessRuntime: Send + Sync {
    /// Execute a command with the given arguments and optional stdin.
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError>;
}

#[cfg(test)]
mod tests;
