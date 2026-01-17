//! Tooling integrations and performance helpers.

pub mod performance;
pub mod perl_critic;
pub mod perltidy;
pub mod subprocess_runtime;

pub use subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};

#[cfg(not(target_arch = "wasm32"))]
pub use subprocess_runtime::OsSubprocessRuntime;
