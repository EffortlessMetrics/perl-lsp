//! Timeout-safe command execution wrapper.
//!
//! Provides a helper to run `std::process::Command` with a wall-clock
//! timeout enforced via a background thread.  When the timeout expires
//! the function returns an `Err`; the spawned process is left to the OS
//! to clean up (it will be reaped when the thread eventually finishes or
//! the LSP process exits).

use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant};

/// Execute `cmd` with a wall-clock `timeout_secs` limit.
///
/// Returns `Ok(Output)` if the command finishes within the timeout, or
/// `Err(String)` with a human-readable message if it times out or fails
/// to spawn.
pub fn run_command_with_timeout(cmd: Command, timeout_secs: u64) -> Result<Output, String> {
    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();

    // Move the command into a thread so we can poll without blocking.
    let join_handle = thread::spawn(move || {
        // cmd is moved here; the binding must be mut for .output()
        let mut cmd = cmd;
        cmd.output()
    });

    loop {
        if start.elapsed() >= timeout {
            // The background thread may still be running; we deliberately
            // do not join it — the process will be reaped by the OS.
            return Err(format!("command timed out after {} seconds", timeout_secs));
        }

        if join_handle.is_finished() {
            return join_handle
                .join()
                .map_err(|_| "command thread panicked".to_string())?
                .map_err(|e| format!("command failed to start: {}", e));
        }

        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn unit_timeout_fires_for_slow_command() {
        let start = Instant::now();
        let mut cmd = Command::new("sleep");
        cmd.arg("10");
        let result = run_command_with_timeout(cmd, 1);
        let elapsed = start.elapsed();

        assert!(result.is_err(), "expected timeout error");
        // Should take approximately 1s, allow up to 4s for slow CI
        assert!(elapsed.as_secs() < 4, "timeout took too long: {}ms", elapsed.as_millis());
    }

    #[test]
    fn unit_fast_command_succeeds() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let result = run_command_with_timeout(cmd, 10);

        assert!(result.is_ok(), "expected success, got: {:?}", result.err());
        if let Ok(output) = result {
            assert!(output.status.success());
        }
    }
}
