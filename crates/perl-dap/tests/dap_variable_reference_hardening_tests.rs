//! Hardening tests for `handle_variables` against invalid and stale
//! `variablesReference` values.
//!
//! # Contract (DAP protocol)
//! An invalid or stale `variablesReference` MUST return:
//!   - `success: true`
//!   - `variables: []` (honest empty, not fabricated placeholders)
//!   - No panic
//!
//! This is protocol-safe per the DAP specification — a client can always
//! request a reference that has become stale (e.g. after a `continue`
//! clears the variable cache) and the adapter must respond gracefully.
//!
//! # Related PRs / issues
//! - Issue #901: DAP variables: invalid/stale variablesReference must not panic

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use perl_tdd_support::must;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn new_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

fn extract_variables_array(body: &serde_json::Value) -> Vec<serde_json::Value> {
    body.get("variables").and_then(|v| v.as_array()).cloned().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// 1. ref = 0  →  protocol-safe empty (success=true, variables=[])
// ---------------------------------------------------------------------------

#[test]
fn test_variables_zero_ref_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(1, "variables", Some(json!({ "variablesReference": 0 })));

    match response {
        DapMessage::Response { success, command, body, message, .. } => {
            assert!(
                success,
                "variablesReference=0 must succeed (protocol-safe empty), got message: {message:?}"
            );
            assert_eq!(command, "variables");
            let body_val = must(body.ok_or("body must be present"));
            let variables = extract_variables_array(&body_val);
            assert!(
                variables.is_empty(),
                "variablesReference=0 must return empty variables, got: {variables:?}"
            );
            Ok(())
        }
        other => must(Err(format!("expected DapMessage::Response, got: {other:?}"))),
    }
}

// ---------------------------------------------------------------------------
// 2. Negative refs → protocol-safe empty
// ---------------------------------------------------------------------------

#[test]
fn test_variables_negative_ref_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();

    for bad_ref in [-1_i64, -10, -100, i32::MIN as i64] {
        let response =
            adapter.handle_request(1, "variables", Some(json!({ "variablesReference": bad_ref })));

        match response {
            DapMessage::Response { success, command, body, message, .. } => {
                assert!(
                    success,
                    "variablesReference={bad_ref} must succeed (protocol-safe empty), got message: {message:?}"
                );
                assert_eq!(command, "variables");
                let body_val = must(body.ok_or(format!("body must be present for ref={bad_ref}")));
                let variables = extract_variables_array(&body_val);
                assert!(
                    variables.is_empty(),
                    "variablesReference={bad_ref} must return empty variables, got: {variables:?}"
                );
            }
            other => {
                return must(Err(format!(
                    "variablesReference={bad_ref}: expected DapMessage::Response, got: {other:?}"
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Huge out-of-range refs → protocol-safe empty (overflow protection)
// ---------------------------------------------------------------------------

#[test]
fn test_variables_out_of_range_ref_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();

    // Values that exceed i32::MAX cannot represent a valid scope encoding.
    let huge_refs: &[i64] = &[i32::MAX as i64 + 1, i64::MAX, i64::MAX - 1, (i32::MAX as i64) * 2];

    for &huge_ref in huge_refs {
        let response =
            adapter.handle_request(1, "variables", Some(json!({ "variablesReference": huge_ref })));

        match response {
            DapMessage::Response { success, command, body, message, .. } => {
                assert!(
                    success,
                    "variablesReference={huge_ref} must succeed (protocol-safe empty), got message: {message:?}"
                );
                assert_eq!(command, "variables");
                let body_val = must(body.ok_or(format!("body must be present for ref={huge_ref}")));
                let variables = extract_variables_array(&body_val);
                assert!(
                    variables.is_empty(),
                    "variablesReference={huge_ref} must return empty variables, got: {variables:?}"
                );
            }
            other => {
                return must(Err(format!(
                    "variablesReference={huge_ref}: expected DapMessage::Response, got: {other:?}"
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. i32::MAX (boundary value) → no panic, success=true
//    i32::MAX = 2147483647 cannot be a valid scope ref (frame_id*10+k would
//    require frame_id > 200M which is nonsensical) but must not panic.
// ---------------------------------------------------------------------------

#[test]
fn test_variables_i32_max_ref_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();

    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({ "variablesReference": i32::MAX as i64 })),
    );

    // i32::MAX is inside the [1, i32::MAX] acceptance range so we go through the
    // normal path. It is an astronomically large frame_id that the debugger will
    // never produce, so we get an honest empty or fallback response.
    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "variablesReference=i32::MAX must not return success=false");
            assert_eq!(command, "variables");
            Ok(())
        }
        other => must(Err(format!("expected DapMessage::Response, got: {other:?}"))),
    }
}

// ---------------------------------------------------------------------------
// 5. Without session: valid-looking refs must not panic
//    The stale-ref guard (Running state) is exercised by the integration
//    lifecycle tests. This covers the no-session path for all ref shapes.
// ---------------------------------------------------------------------------

#[test]
fn test_variables_without_session_valid_ref_does_not_panic()
-> Result<(), Box<dyn std::error::Error>> {
    // Without an active session the adapter must respond, not panic, for any ref.
    let mut adapter = new_adapter();

    let stable_refs: &[i64] = &[1, 11, 12, 13, 21, 100, 1000];

    for &var_ref in stable_refs {
        let response =
            adapter.handle_request(1, "variables", Some(json!({ "variablesReference": var_ref })));

        match response {
            DapMessage::Response { success, command, .. } => {
                assert!(
                    success,
                    "variablesReference={var_ref} without session must succeed, not panic"
                );
                assert_eq!(command, "variables");
            }
            other => {
                return must(Err(format!(
                    "variablesReference={var_ref}: expected DapMessage::Response, got: {other:?}"
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Boundary: ref = 1 (smallest valid encoding)
// ---------------------------------------------------------------------------

#[test]
fn test_variables_ref_one_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(1, "variables", Some(json!({ "variablesReference": 1 })));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "variablesReference=1 must not return success=false");
            assert_eq!(command, "variables");
            assert!(body.is_some(), "body must be present");
            Ok(())
        }
        other => must(Err(format!("expected DapMessage::Response, got: {other:?}"))),
    }
}

// ---------------------------------------------------------------------------
// 7. Boundary: ref = i32::MAX + 1 (just above the accepted ceiling)
// ---------------------------------------------------------------------------

#[test]
fn test_variables_just_above_i32_max_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let just_over = i32::MAX as i64 + 1;

    let response =
        adapter.handle_request(1, "variables", Some(json!({ "variablesReference": just_over })));

    match response {
        DapMessage::Response { success, command, body, message, .. } => {
            assert!(
                success,
                "variablesReference={just_over} must return success=true (protocol-safe empty), got message: {message:?}"
            );
            assert_eq!(command, "variables");
            let body_val = must(body.ok_or("body must be present"));
            let variables = extract_variables_array(&body_val);
            assert!(
                variables.is_empty(),
                "variablesReference={just_over} must return empty variables, got: {variables:?}"
            );
            Ok(())
        }
        other => must(Err(format!("expected DapMessage::Response, got: {other:?}"))),
    }
}

// ---------------------------------------------------------------------------
// 8. Stale-ref guard: session in Running state returns protocol-safe empty
//
// This exercises the new code block added in PR #901:
//   if session.state != DebugState::Stopped { return success=true, variables=[] }
//
// Without this test the "session is not stopped" branch (Running state) is
// exercised only by integration lifecycle tests, leaving patch coverage below 95%.
// ---------------------------------------------------------------------------

/// Skip if perl is not on PATH (seed_running_session_for_test spawns `perl -e 1`).
fn perl_available() -> bool {
    std::process::Command::new("perl").arg("-e").arg("1").output().is_ok()
}

#[test]
fn test_variables_with_running_session_returns_protocol_safe_empty()
-> Result<(), Box<dyn std::error::Error>> {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = new_adapter();
    // Seed a session in Running state — the stale-ref guard fires for any valid ref.
    adapter.seed_running_session_for_test();

    // Use a valid-looking scope ref (frame_id=1, scope_type=1 → 11).
    let response =
        adapter.handle_request(1, "variables", Some(json!({ "variablesReference": 11 })));

    match response {
        DapMessage::Response { success, command, body, message, .. } => {
            assert!(
                success,
                "variables with Running session must return success=true (stale-ref guard), got message: {message:?}"
            );
            assert_eq!(command, "variables");
            let body_val = must(body.ok_or("body must be present for Running-state guard"));
            let variables = extract_variables_array(&body_val);
            assert!(
                variables.is_empty(),
                "variables with Running session must return empty variables (stale cache), got: {variables:?}"
            );
            Ok(())
        }
        other => {
            must(Err(format!("stale-ref guard: expected DapMessage::Response, got: {other:?}")))
        }
    }
}

#[test]
fn test_variables_running_session_multiple_valid_refs_all_empty()
-> Result<(), Box<dyn std::error::Error>> {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = new_adapter();
    adapter.seed_running_session_for_test();

    // Multiple valid-looking refs — all should be stale-guarded.
    let valid_refs: &[i64] = &[1, 11, 12, 13, 21, 22, 23, 100, 1000];
    for &var_ref in valid_refs {
        let response =
            adapter.handle_request(1, "variables", Some(json!({ "variablesReference": var_ref })));
        match response {
            DapMessage::Response { success, command, body, .. } => {
                assert!(
                    success,
                    "variablesReference={var_ref} with Running session must succeed (stale-ref guard)"
                );
                assert_eq!(command, "variables");
                let body_val = must(body.ok_or(format!("body must be present for ref={var_ref}")));
                let variables = extract_variables_array(&body_val);
                assert!(
                    variables.is_empty(),
                    "variablesReference={var_ref} with Running session must be empty, got: {variables:?}"
                );
            }
            other => {
                return must(Err(format!(
                    "variablesReference={var_ref}: expected Response, got: {other:?}"
                )));
            }
        }
    }
    Ok(())
}
