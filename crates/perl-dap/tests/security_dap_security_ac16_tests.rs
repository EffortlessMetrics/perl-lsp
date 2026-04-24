//! DAP Security AC16 — Defensive integration tests
//!
//! Verifies that existing protections in `perl-dap-security` correctly harden
//! the three attack surfaces the DAP server is most exposed to:
//!
//! 1. **Breakpoint path traversal** — malicious `source.path` values in
//!    `setBreakpoints` requests that try to escape the workspace.
//! 2. **Eval expression injection** — newline/CR payloads in `evaluate`
//!    requests that try to inject additional protocol frames.
//! 3. **Resource limit enforcement** — timeout values above the server cap
//!    that could allow denial-of-service via infinite-loop debug targets.
//!
//! These tests exercise the public API of `perl_dap_security` as the DAP
//! server would call it when processing incoming protocol messages.
//!
//! Spec: docs/DAP_SECURITY_SPECIFICATION.md
//! AC:16

use perl_dap::security::{
    MAX_TIMEOUT_MS, SecurityError, validate_condition, validate_expression, validate_path,
    validate_timeout,
};
use std::path::Path;

type R = Result<(), Box<dyn std::error::Error>>;

// ===========================================================================
// Section 1: Breakpoint path traversal (setBreakpoints source.path)
// ===========================================================================

/// Classic `../../etc/passwd` breakpoint path must be rejected.
#[test]
fn breakpoint_path_classic_parent_traversal_rejected() -> R {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().canonicalize()?;
    let result = validate_path(Path::new("../../etc/passwd"), &ws);
    assert!(result.is_err(), "Classic traversal must be rejected");
    match result {
        Err(SecurityError::PathTraversalAttempt(_))
        | Err(SecurityError::PathOutsideWorkspace(_)) => {}
        Err(e) => return Err(format!("Wrong error kind: {e:?}").into()),
        Ok(_) => return Err("Expected error, got Ok".into()),
    }
    Ok(())
}

/// An absolute path to a system file must be rejected even if it looks safe.
#[test]
fn breakpoint_path_absolute_system_file_rejected() -> R {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().canonicalize()?;
    let result = validate_path(Path::new("/etc/passwd"), &ws);
    assert!(result.is_err(), "Absolute path outside workspace must be rejected");
    Ok(())
}

/// A null byte injected into a breakpoint path must be rejected.
#[test]
fn breakpoint_path_null_byte_injection_rejected() -> R {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().canonicalize()?;
    let result = validate_path(Path::new("src/main.pl\0.evil"), &ws);
    assert!(
        matches!(result, Err(SecurityError::InvalidPathCharacters)),
        "Null byte in breakpoint path must produce InvalidPathCharacters, got: {result:?}"
    );
    Ok(())
}

/// A breakpoint path that stays within the workspace is accepted.
#[test]
fn breakpoint_path_within_workspace_accepted() -> R {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().canonicalize()?;
    let result = validate_path(Path::new("src/main.pl"), &ws)?;
    assert!(result.starts_with(&ws), "Resolved path must be within workspace");
    Ok(())
}

/// A deeply nested traversal (50 levels) must be rejected — prevents path
/// bomb attacks where attackers chain many `..` segments hoping validation
/// gives up or overflows.
#[test]
fn breakpoint_path_deep_traversal_bomb_rejected() -> R {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().canonicalize()?;
    let payload = "../".repeat(50) + "etc/passwd";
    let result = validate_path(Path::new(&payload), &ws);
    assert!(result.is_err(), "Deep traversal bomb must be rejected");
    Ok(())
}

/// A breakpoint path pointing at `/proc/self/environ` must be rejected.
/// This is a real attack vector when the debugger runs with elevated privileges.
#[test]
fn breakpoint_path_proc_self_environ_rejected() -> R {
    let tmp = tempfile::tempdir()?;
    let ws = tmp.path().canonicalize()?;
    let result = validate_path(Path::new("/proc/self/environ"), &ws);
    assert!(result.is_err(), "/proc/self/environ must be rejected as breakpoint path");
    Ok(())
}

// ===========================================================================
// Section 2: Expression injection (evaluate / setBreakpoints condition)
// ===========================================================================

/// A newline in an `evaluate` expression is a protocol injection attempt:
/// the injected text after the newline would appear as a new DAP message body.
#[test]
fn eval_expression_newline_injection_rejected() -> R {
    let result = validate_expression("$x + 1\nContent-Length: 100\r\n\r\n{\"injected\":true}");
    assert!(
        matches!(result, Err(SecurityError::InvalidExpression)),
        "Newline injection in eval expression must be rejected"
    );
    Ok(())
}

/// A carriage-return-only injection in an `evaluate` expression must be rejected.
#[test]
fn eval_expression_cr_injection_rejected() -> R {
    let result = validate_expression("$x + 1\rsystem('id')");
    assert!(
        matches!(result, Err(SecurityError::InvalidExpression)),
        "CR injection in eval expression must be rejected"
    );
    Ok(())
}

/// A CRLF sequence must be rejected — DAP uses CRLF as a header terminator.
#[test]
fn eval_expression_crlf_injection_rejected() -> R {
    let result = validate_expression("1\r\n");
    assert!(
        matches!(result, Err(SecurityError::InvalidExpression)),
        "CRLF in eval expression must be rejected"
    );
    Ok(())
}

/// Valid single-line expressions (no injection) must be accepted.
#[test]
fn eval_expression_valid_single_line_accepted() -> R {
    validate_expression("$x + 1")?;
    validate_expression("scalar @array")?;
    validate_expression("defined($var) && $var > 0")?;
    validate_expression("$hash{key}")?;
    Ok(())
}

/// A newline injected into a breakpoint condition must be rejected.
/// Conditions are passed directly to the debugger and a newline would
/// terminate the condition and inject a new debugger command.
#[test]
fn breakpoint_condition_newline_injection_rejected() -> R {
    let result = validate_condition("$x > 10\nsystem('id')");
    assert!(
        matches!(result, Err(SecurityError::InvalidExpression)),
        "Newline injection in breakpoint condition must be rejected"
    );
    Ok(())
}

/// Valid breakpoint conditions must be accepted.
#[test]
fn breakpoint_condition_valid_accepted() -> R {
    validate_condition("$x > 10")?;
    validate_condition("defined($var)")?;
    validate_condition("$i == 42 && $j < 100")?;
    Ok(())
}

// ===========================================================================
// Section 3: Resource limit enforcement (timeout)
// ===========================================================================

/// A timeout exceeding the server cap must be rejected.
/// Without this, a client could set a 24-hour timeout, keeping a debug
/// session open and consuming debugger resources indefinitely.
#[test]
fn resource_limit_excessive_timeout_rejected() {
    let result = validate_timeout(MAX_TIMEOUT_MS + 1);
    assert!(
        matches!(result, Err(SecurityError::ExcessiveTimeout(_))),
        "Timeout above cap must be rejected"
    );
}

/// The maximum allowed timeout (exactly at the cap) must be accepted.
#[test]
fn resource_limit_max_timeout_exactly_at_cap_accepted() -> R {
    let clamped = validate_timeout(MAX_TIMEOUT_MS)?;
    assert_eq!(clamped, MAX_TIMEOUT_MS);
    Ok(())
}

/// A zero timeout must be clamped to 1ms — the server never blocks forever
/// but also never uses a nonsensical zero-duration timeout.
#[test]
fn resource_limit_zero_timeout_clamped_to_one() -> R {
    let clamped = validate_timeout(0)?;
    assert_eq!(clamped, 1, "Zero timeout must be clamped to 1ms");
    Ok(())
}

/// An extremely large timeout (u32::MAX) must be rejected.
#[test]
fn resource_limit_u32_max_timeout_rejected() {
    let result = validate_timeout(u32::MAX);
    assert!(
        matches!(result, Err(SecurityError::ExcessiveTimeout(_))),
        "u32::MAX timeout must be rejected"
    );
}

/// Normal operational timeouts (1s, 5s, 60s) must all be accepted unchanged.
#[test]
fn resource_limit_normal_timeouts_accepted_unchanged() -> R {
    assert_eq!(validate_timeout(1_000)?, 1_000);
    assert_eq!(validate_timeout(5_000)?, 5_000);
    assert_eq!(validate_timeout(60_000)?, 60_000);
    Ok(())
}

/// The cap value itself must be 5 minutes (300,000ms). This is the
/// documented security boundary and must not silently change.
#[test]
fn resource_limit_cap_is_five_minutes() {
    assert_eq!(MAX_TIMEOUT_MS, 300_000, "Security cap must be 5 minutes (300,000ms)");
}
