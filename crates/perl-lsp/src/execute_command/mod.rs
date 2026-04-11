//! Execute command support for running tests and debugging.
//!
//! This module provides comprehensive executeCommand support for the Perl Language Server,
//! implementing a dual analyzer strategy that combines external tool integration with
//! built-in fallback analysis. The implementation ensures 100% availability and robust
//! security through workspace root enforcement and path traversal protection.

mod executor;
mod provider;
#[cfg(any(test, doctest))]
mod test_support;
#[cfg(test)]
mod tests;
mod types;

pub use executor::CommandExecutor;
pub use provider::{ExecuteCommandProvider, command_exists, get_supported_commands};
pub(crate) use provider::normalize_path_for_external_command;
pub use types::{CommandResult, PerlCommand};
