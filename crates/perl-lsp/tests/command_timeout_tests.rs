//! Tests for the command timeout utility.
//!
//! Validates that `run_command_with_timeout` enforces timeouts and
//! passes through successful command output correctly.

use perl_lsp::util::run_command_with_timeout;
use std::process::Command;
use std::time::Instant;

#[test]
fn test_command_timeout_fires() {
    let start = Instant::now();
    let mut cmd = Command::new("sleep");
    cmd.arg("5");
    let result = run_command_with_timeout(cmd, 1);
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Expected timeout error but got: {:?}", result.ok());
    assert!(
        elapsed.as_secs() >= 1 && elapsed.as_secs() <= 3,
        "Expected ~1s timeout, got {}s",
        elapsed.as_secs()
    );
}

#[test]
fn test_command_completes_before_timeout() {
    let mut cmd = Command::new("echo");
    cmd.arg("hello");
    let result = run_command_with_timeout(cmd, 10);

    assert!(result.is_ok(), "Expected success but got: {:?}", result.err());
    if let Ok(output) = result {
        assert!(output.status.success());
    }
}
