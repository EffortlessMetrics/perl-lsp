//! Integration tests for #1338: stale eval_ref cache-miss short-circuit after resume.
//!
//! **Bug**: On resume (continue/next/step), `variable_cache.clear()` runs, making
//! outstanding eval_ref variablesReferences (in the range [1_000_000, 1_999_999_999])
//! stale. A subsequent variables request for a stale eval_ref would fall through to
//! scope routing and call `parse_scope_variables_from_output()` — wasteful and
//! semantically wrong (an eval_ref is NOT a scope; scope routing must not run for it).
//!
//! **Fix (PR #1338)**: In `handle_variables()`, after the VariableReference codec
//! decode in the cache-miss branch, add an early short-circuit: if the decoded variant
//! is `EvalResult`, return `success=true, variables=[]` immediately, before any scope
//! routing or debugger query logic executes.
//!
//! # Test design notes
//!
//! The spec asked for a test that "FAILS without the fix and PASSES with it". After
//! thorough investigation of the code path, this is documented honestly:
//!
//! ## Why a strict fail-without-pass-with test is not achievable here
//!
//! Without the fix, the code path for (Stopped session + eval_ref + cache miss) is:
//!   1. Pass the Running-state guard (session IS Stopped)
//!   2. Cache miss -> enter else branch
//!   3. Decode(eval_ref wire) -> EvalResult -> scope_kind = None
//!   4. match scope_kind { None => {} } -- no-op (no scope query sent)
//!   5. framed_scope_lines = None -> call wait_for_debugger_output_window(75ms)
//!   6. Call parse_scope_variables_from_output(eval_ref_wire, ...)
//!   7. parse_scope_variables_from_lines with EvalResult ref -> returns empty (line 134)
//!   8. full_roots.is_empty() -> fallback_scope_variables -> empty for EvalResult
//!   9. Response: success=true, variables=[] -- same as with the fix!
//!
//! The observable response is identical with or without the fix. The difference is:
//! - WITHOUT fix: 75ms delay (wait_for_debugger_output_window), wrong code path
//! - WITH fix: immediate return, correct code path
//!
//! A timing-based test would be fragile (CI latency variance). Instead, this test
//! suite provides:
//!   (A) Active-session tests that exercise the Stopped-session paths
//!   (B) Codec invariant tests verifying the band classification the fix relies on
//!   (C) Protocol-contract tests verifying correct response shape in all cases
//!   (D) A "scope ref does not short-circuit" test verifying the fix is scoped to
//!       EvalResult refs only (not Scope refs, which must still go through routing)
//!
//! The fix is verified by code-reading: the early short-circuit at variables.rs
//! after the decode is the correct place to gate stale EvalResult refs.
//! Flagged for deep-review to scrutinize the fix correctness via code inspection.

#![allow(clippy::expect_used)]

use perl_dap::debug_adapter::var_ref::VariableReference;
use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper: check if perl is available on PATH.
fn perl_available() -> bool {
    std::process::Command::new("perl").arg("-e").arg("1").output().is_ok()
}

/// Helper: returns true if the response is success=true with empty variables.
fn is_honest_empty(msg: &DapMessage) -> bool {
    match msg {
        DapMessage::Response { success, body, message, .. } => {
            *success
                && message.is_none()
                && body
                    .as_ref()
                    .and_then(|b| b.get("variables"))
                    .and_then(|v| v.as_array())
                    .map(|a| a.is_empty())
                    .unwrap_or(false)
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Codec invariant: wire-band classification (no active session needed)
// ---------------------------------------------------------------------------

/// Protocol contract: eval_ref wires in the EvalResult band [1_000_000, 1_999_999_999]
/// must decode as EvalResult, NOT as Scope. This is the foundation of the fix.
#[test]
fn eval_ref_wire_decodes_as_eval_result_not_scope() -> TestResult {
    for wire in [1_000_000_i32, 1_000_001, 1_000_003, 1_100_000, 1_999_999_999] {
        let decoded = VariableReference::decode(wire);
        assert!(
            matches!(decoded, Some(VariableReference::EvalResult { .. })),
            "eval_ref wire {wire} must decode as EvalResult, got: {decoded:?}"
        );
    }
    Ok(())
}

/// Protocol contract: scope wires in [1, 999_999] with kind in {1,2,3} decode as Scope.
/// Verifies that the bands are disjoint and the decode is correct for scope refs.
#[test]
fn scope_wire_decodes_as_scope_not_eval_result() -> TestResult {
    for wire in [11_i32, 12, 13, 21, 22, 23, 101] {
        let decoded = VariableReference::decode(wire);
        assert!(
            matches!(decoded, Some(VariableReference::Scope { .. })),
            "scope wire {wire} must decode as Scope, got: {decoded:?}"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// No-session baseline (exercises the session-state guard, not the fix)
// ---------------------------------------------------------------------------

/// Baseline: without any session, a stale eval_ref returns honest empty via the
/// no-session path (not the fix path -- the fix is in the Stopped-session branch).
#[test]
fn stale_eval_ref_no_session_returns_honest_empty() -> TestResult {
    let mut adapter = DebugAdapter::new();
    let stale_eval_ref_wire: i64 = 1_000_001;

    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({ "variablesReference": stale_eval_ref_wire })),
    );

    assert!(
        is_honest_empty(&response),
        "stale eval_ref without session must return honest empty; got: {response:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Active-session tests: Stopped session + stale eval_ref (exercises the fix path)
// ---------------------------------------------------------------------------

/// **Core regression test** (Issue #1338):
/// Active Stopped session + stale eval_ref (cache miss after resume) -> honest empty.
///
/// This exercises the fix path:
///   1. Session IS Stopped -> passes the Running-state guard (lines 73-92)
///   2. Cache miss for eval_ref wire -> enters the else branch (line 105)
///   3. Fix: decode is EvalResult -> short-circuit, return honest empty immediately
///
/// Without the fix: the code would take a 75ms detour through scope routing
/// (wait_for_debugger_output_window + parse_scope_variables_from_output) before
/// ultimately returning empty via fallback_scope_variables -- same output, wrong path.
///
/// With the fix: returns immediately without any scope routing or delay.
///
/// NOTE: Because both paths return success=true + empty variables, this test passes
/// both before and after the fix in terms of observable output. The fix is verified
/// by code-reading (see module doc). The test documents the protocol contract and
/// ensures the Stopped-session path is exercised.
#[test]
fn active_stopped_session_stale_eval_ref_returns_honest_empty() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = DebugAdapter::new();

    // Seed a Stopped session (passes the Running-state guard).
    // This is the critical difference from the no-session test above.
    adapter.seed_stopped_session_with_frames_for_test(vec![]);

    // Stale eval_ref wire: EvalResult band, not in cache (cache was cleared on resume).
    let stale_eval_ref_wire: i64 = 1_000_001;

    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({ "variablesReference": stale_eval_ref_wire })),
    );

    assert!(
        is_honest_empty(&response),
        "Stopped session + stale eval_ref must return honest empty (success=true, variables=[]); \
         got: {response:?}"
    );
    Ok(())
}

/// Multiple stale eval_refs (different counters) all return honest empty.
///
/// Verifies the short-circuit covers the entire EvalResult band, not just counter=1.
#[test]
fn active_stopped_session_multiple_stale_eval_refs_all_honest_empty() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = DebugAdapter::new();
    adapter.seed_stopped_session_with_frames_for_test(vec![]);

    // Range of eval_ref wire values across the EvalResult band.
    let stale_eval_refs: &[i64] =
        &[1_000_000, 1_000_001, 1_000_003, 1_001_000, 1_100_000, 1_999_999_999];

    for &eval_ref_wire in stale_eval_refs {
        let response = adapter.handle_request(
            1,
            "variables",
            Some(json!({ "variablesReference": eval_ref_wire })),
        );
        assert!(
            is_honest_empty(&response),
            "Stopped session + stale eval_ref wire {eval_ref_wire} must return honest empty; \
             got: {response:?}"
        );
    }
    Ok(())
}

/// Scope refs with a Stopped session do NOT get short-circuited by the eval_ref fix.
/// This verifies the fix is narrowly scoped to EvalResult refs only.
///
/// Scope refs must still go through scope routing (not the short-circuit path).
/// Expected: scope refs return success=true (they may return fallback scope data or
/// empty, but never error; the session path is exercised for scopes).
#[test]
fn active_stopped_session_scope_ref_not_affected_by_eval_ref_fix() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = DebugAdapter::new();
    adapter.seed_stopped_session_with_frames_for_test(vec![]);

    // Scope ref: frame_id=1, Locals -> wire 11 (well within Scope band)
    let scope_ref_wire: i64 = 11;

    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({ "variablesReference": scope_ref_wire })),
    );

    // Must succeed (never error) -- the fix must NOT break scope refs.
    match &response {
        DapMessage::Response { success, .. } => {
            assert!(
                *success,
                "Stopped session + scope ref must succeed (not error); got: {response:?}"
            );
        }
        other => {
            return Err(format!("expected DapMessage::Response, got: {other:?}").into());
        }
    }
    Ok(())
}

/// Running session + stale eval_ref -> honest empty via the Running-state guard
/// (NOT the fix path, but verifies the two guards compose correctly).
#[test]
fn running_session_stale_eval_ref_returns_honest_empty_via_running_guard() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = DebugAdapter::new();

    // Running state: the stale-ref guard at lines 73-92 fires first.
    adapter.seed_running_session_for_test();

    let stale_eval_ref_wire: i64 = 1_000_001;
    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({ "variablesReference": stale_eval_ref_wire })),
    );

    assert!(
        is_honest_empty(&response),
        "Running session + eval_ref must return honest empty via stale-ref guard; \
         got: {response:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Response shape contract tests
// ---------------------------------------------------------------------------

/// Response shape: the variables array must be present (not null/missing) and empty.
/// Tests the DAP protocol shape contract -- not just success=true.
#[test]
fn stale_eval_ref_response_has_correct_dap_shape() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let mut adapter = DebugAdapter::new();
    adapter.seed_stopped_session_with_frames_for_test(vec![]);

    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({ "variablesReference": 1_000_001_i64 })),
    );

    match response {
        DapMessage::Response { success, body, command, message, .. } => {
            assert!(success, "must succeed");
            assert_eq!(command, "variables", "command must be 'variables'");
            assert!(message.is_none(), "must not have error message");
            let body = body.expect("body must be present");
            let vars = body.get("variables").expect("body must have 'variables' key");
            let arr = vars.as_array().expect("variables must be an array");
            assert!(arr.is_empty(), "variables array must be empty for stale eval_ref");
        }
        other => {
            return Err(format!("expected DapMessage::Response, got: {other:?}").into());
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pre-existing regression tests (from red-TDD commits 96cb7040f + c5a5a2e0d)
// ---------------------------------------------------------------------------

/// Regression test from red-TDD: eval_ref without session returns honest empty.
/// Confirms the no-session path still works after the fix is applied.
#[test]
fn test_eval_ref_without_session_returns_honest_empty() -> TestResult {
    let mut adapter = DebugAdapter::new();

    let stale_eval_ref_wire = 1_000_001i32;

    assert!(
        matches!(
            VariableReference::decode(stale_eval_ref_wire),
            Some(VariableReference::EvalResult { counter: 1 })
        ),
        "setup: eval_ref wire {stale_eval_ref_wire} must decode as EvalResult"
    );

    let response = adapter.handle_request(
        1,
        "variables",
        Some(json!({
            "variablesReference": stale_eval_ref_wire
        })),
    );

    match response {
        DapMessage::Response { success, body, message, .. } => {
            assert!(
                success,
                "stale eval_ref must succeed (not error) and return empty. \
                 Message: {message:?}"
            );
            assert!(message.is_none(), "stale eval_ref must not have error message: {message:?}");

            let vars_list = body
                .and_then(|b| b.get("variables").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            assert!(
                vars_list.is_empty(),
                "stale eval_ref must return empty variables: {vars_list:?}"
            );
        }
        other => {
            return Err(format!("expected Response message, got: {other:?}").into());
        }
    }

    Ok(())
}

/// Regression test from red-TDD: protocol contract -- eval_ref wires are never
/// misinterpreted as scope refs in scope routing logic.
#[test]
fn test_eval_ref_never_misinterpreted_as_scope_in_protocol() -> TestResult {
    let eval_ref_wire = 1_000_001i32;

    match VariableReference::decode(eval_ref_wire) {
        Some(VariableReference::EvalResult { counter: 1 }) => {
            // Correct: eval band recognized
        }
        Some(VariableReference::Scope { frame_id, .. }) => {
            return Err(format!(
                "PROTOCOL BUG: eval_ref wire {eval_ref_wire} was misclassified as Scope \
                   with frame_id={frame_id}. This is the old bug that led to scope routing \
                   for eval refs."
            )
            .into());
        }
        other => {
            return Err(format!(
                "unexpected decode result for eval_ref wire {eval_ref_wire}: {other:?}"
            )
            .into());
        }
    }

    Ok(())
}
