//! OS-backed subprocess runtime.

mod invocation;
mod process;
mod validation;
#[cfg(windows)]
mod windows;

pub(crate) use invocation::resolve_command_invocation;
use process::run_os_command;

use crate::{SubprocessError, SubprocessOutput, SubprocessRuntime};

#[cfg(all(windows, test))]
pub(crate) use windows::{windows_program_priority, windows_quote_for_cmd};

/// Default implementation using `std::process::Command`.
pub struct OsSubprocessRuntime {
    timeout_secs: Option<u64>,
}

impl OsSubprocessRuntime {
    /// Create a new OS subprocess runtime with no timeout.
    pub fn new() -> Self {
        Self { timeout_secs: None }
    }

    /// Create a new OS subprocess runtime with the given wall-clock timeout.
    ///
    /// If the subprocess does not complete within `timeout_secs` seconds the
    /// call returns a `SubprocessError` with a "timed out" message and attempts
    /// to terminate the spawned process before returning.
    ///
    /// # Stdin size caveat
    ///
    /// Stdin data is written synchronously before the timeout poll loop begins.
    /// If the subprocess hangs before consuming stdin and the data exceeds the
    /// OS pipe buffer (~64 KiB on Linux), `run_command` will block in the write
    /// phase and the timeout will not fire. For typical Perl source files this
    /// is not a concern.
    ///
    /// # Panics
    ///
    /// Panics if `timeout_secs` is zero (a zero-second timeout would time out
    /// every command immediately and is almost certainly a caller bug).
    pub fn with_timeout(timeout_secs: u64) -> Self {
        assert!(timeout_secs > 0, "timeout_secs must be greater than zero");
        Self { timeout_secs: Some(timeout_secs) }
    }
}

impl Default for OsSubprocessRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SubprocessRuntime for OsSubprocessRuntime {
    fn run_command(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<SubprocessOutput, SubprocessError> {
        run_os_command(program, args, stdin, self.timeout_secs)
    }
}
