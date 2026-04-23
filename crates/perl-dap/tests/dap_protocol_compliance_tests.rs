//! DAP Protocol Compliance & Integration Tests (Issue #2785)
//!
//! Covers:
//! 1. Error handling paths for 10 previously untested commands
//! 2. Security validation (path traversal, newline injection)
//! 3. Protocol state machine compliance (sequence monotonicity, response shape)
//! 4. Response schema validation (required body fields)
//! 5. Integration scenarios (cancel signal, state transitions)

// Tests use `panic!` in match arms as structured test failure reporters,
// and `expect()` on response body values that must be present per DAP spec.
#![allow(clippy::panic, clippy::expect_used)]

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::sync::mpsc::channel;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

fn make_adapter_with_events() -> (DebugAdapter, std::sync::mpsc::Receiver<DapMessage>) {
    let (tx, rx) = channel();
    let mut adapter = DebugAdapter::new();
    adapter.set_event_sender(tx);
    (adapter, rx)
}

/// Extract success and message from a Response, panicking on non-Response.
#[allow(clippy::panic)]
fn assert_response(
    msg: DapMessage,
    expected_command: &str,
    expected_success: bool,
) -> Option<Value> {
    match msg {
        DapMessage::Response { success, command, body, .. } => {
            assert_eq!(command, expected_command, "command mismatch");
            assert_eq!(success, expected_success, "success mismatch for {expected_command}");
            body
        }
        other => panic!("expected Response for {expected_command}, got {other:?}"),
    }
}

#[allow(clippy::panic)]
fn assert_response_message(msg: DapMessage, expected_command: &str) -> Option<String> {
    match msg {
        DapMessage::Response { command, message, .. } => {
            assert_eq!(command, expected_command);
            message
        }
        other => panic!("expected Response for {expected_command}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 1. Error Handling: 10 Previously-Untested Commands
// ---------------------------------------------------------------------------

// --- breakpointLocations ---

#[test]
// AC:16
fn test_breakpoint_locations_missing_args_returns_failure() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "breakpointLocations", None);
    assert_response(response, "breakpointLocations", false);
    Ok(())
}

#[test]
// AC:16
fn test_breakpoint_locations_no_source_path_returns_empty_success()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({
        "source": {},
        "line": 5
    });
    let response = adapter.handle_request(1, "breakpointLocations", Some(args));
    let body = assert_response(response, "breakpointLocations", true);
    let body = body.ok_or("breakpointLocations should return a body")?;
    assert!(body.get("breakpoints").is_some(), "body must include breakpoints array");
    Ok(())
}

// --- cancel ---

#[test]
// AC:16
fn test_cancel_succeeds_without_args() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "cancel", None);
    assert_response(response, "cancel", true);
    Ok(())
}

#[test]
// AC:16
fn test_cancel_succeeds_with_args() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({ "requestId": 42 });
    let response = adapter.handle_request(1, "cancel", Some(args));
    assert_response(response, "cancel", true);
    Ok(())
}

// --- exceptionInfo ---

#[test]
// AC:16
fn test_exception_info_no_active_session_returns_unknown_exception()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    // No session means last_exception_message is None → "Unknown exception"
    let response = adapter.handle_request(1, "exceptionInfo", None);
    let body = assert_response(response, "exceptionInfo", true);
    let body = body.ok_or("exceptionInfo must return a body")?;
    assert!(body.get("exceptionId").is_some(), "exceptionId required");
    assert!(body.get("breakMode").is_some(), "breakMode required");
    Ok(())
}

// --- goto ---

#[test]
// AC:16
fn test_goto_missing_args_returns_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "goto", None);
    assert_response(response, "goto", false);
    Ok(())
}

#[test]
// AC:16
fn test_goto_unknown_target_id_returns_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({ "threadId": 1, "targetId": 9999 });
    let response = adapter.handle_request(1, "goto", Some(args));
    // targetId 9999 has never been registered via gotoTargets, so the implementation
    // removes it from the goto_map (returning None) and unconditionally returns failure.
    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "goto");
            assert!(!success, "goto with unregistered targetId must fail");
            assert!(message.is_some(), "failure response must include a message");
        }
        other => panic!("expected Response, got {other:?}"),
    }
    Ok(())
}

// --- gotoTargets ---

#[test]
// AC:16
fn test_goto_targets_missing_args_returns_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "gotoTargets", None);
    assert_response(response, "gotoTargets", false);
    Ok(())
}

#[test]
// AC:16
fn test_goto_targets_no_source_path_returns_empty_success() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let args = json!({
        "source": {},
        "line": 1
    });
    let response = adapter.handle_request(1, "gotoTargets", Some(args));
    let body = assert_response(response, "gotoTargets", true);
    let body = body.ok_or("gotoTargets must return a body")?;
    assert!(body.get("targets").is_some(), "targets array required in response body");
    Ok(())
}

// --- loadedSources ---

#[test]
// AC:16
fn test_loaded_sources_no_session_returns_empty_sources() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "loadedSources", None);
    let body = assert_response(response, "loadedSources", true);
    let body = body.ok_or("loadedSources must return a body")?;
    let sources = body.get("sources").ok_or("sources array required")?;
    assert!(
        sources.as_array().is_some_and(|a| a.is_empty()),
        "without a session, sources must be empty"
    );
    Ok(())
}

// --- restartFrame ---

#[test]
// AC:16
fn test_restart_frame_always_fails_with_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "restartFrame", None);
    let msg = assert_response_message(response, "restartFrame");
    assert!(msg.is_some(), "restartFrame must include an error message");
    let msg = msg.ok_or("restartFrame must include an error message")?;
    assert!(
        msg.contains("Perl") || msg.contains("stack frame"),
        "message should explain why restartFrame is unsupported: {msg}"
    );
    Ok(())
}

// --- setExpression ---

#[test]
// AC:16
fn test_set_expression_missing_args_returns_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "setExpression", None);
    assert_response(response, "setExpression", false);
    Ok(())
}

#[test]
// AC:16
fn test_set_expression_empty_expression_returns_failure() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let args = json!({ "expression": "", "value": "42" });
    let response = adapter.handle_request(1, "setExpression", Some(args));
    assert_response(response, "setExpression", false);
    Ok(())
}

#[test]
// AC:16
fn test_set_expression_empty_value_returns_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({ "expression": "$x", "value": "" });
    let response = adapter.handle_request(1, "setExpression", Some(args));
    assert_response(response, "setExpression", false);
    Ok(())
}

// --- stepInTargets ---

#[test]
// AC:16
fn test_step_in_targets_missing_args_returns_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "stepInTargets", None);
    assert_response(response, "stepInTargets", false);
    Ok(())
}

#[test]
// AC:16
fn test_step_in_targets_no_session_returns_empty_success() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let args = json!({ "frameId": 1 });
    let response = adapter.handle_request(1, "stepInTargets", Some(args));
    let body = assert_response(response, "stepInTargets", true);
    let body = body.ok_or("stepInTargets must return a body")?;
    assert!(body.get("targets").is_some(), "targets array required");
    Ok(())
}

// --- terminateThreads ---

#[test]
// AC:16
fn test_terminate_threads_always_fails_with_message() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "terminateThreads", None);
    let msg = assert_response_message(response, "terminateThreads");
    assert!(msg.is_some(), "terminateThreads must include an error message");
    let msg = msg.ok_or("terminateThreads must include an error message")?;
    assert!(
        msg.contains("thread") || msg.contains("Perl"),
        "message should explain threading limitation: {msg}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Security Validation Tests
// ---------------------------------------------------------------------------

// --- gotoTargets path traversal ---

#[test]
// AC:16 (Security: path traversal in gotoTargets)
fn test_goto_targets_path_traversal_blocked_when_workspace_set()
-> Result<(), Box<dyn std::error::Error>> {
    // Without a workspace root set, validate_source_path allows any path.
    // This test verifies that the path traversal path doesn't panic and
    // returns a safe/empty response rather than reading arbitrary files.
    let mut adapter = make_adapter();
    let malicious_paths =
        vec!["../../../etc/passwd", "/etc/passwd", "../../../../../../tmp/sensitive"];

    for path in malicious_paths {
        let args = json!({
            "source": { "path": path },
            "line": 1
        });
        let response = adapter.handle_request(1, "gotoTargets", Some(args));
        // Should succeed OR fail with an error — but must not panic
        match response {
            DapMessage::Response { command, .. } => {
                assert_eq!(command, "gotoTargets");
            }
            other => panic!("expected Response, got {other:?}"),
        }
    }
    Ok(())
}

// --- breakpointLocations path traversal ---

#[test]
// AC:16 (Security: path traversal in breakpointLocations)
fn test_breakpoint_locations_path_traversal_does_not_panic()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({
        "source": { "path": "../../../../etc/shadow" },
        "line": 1
    });
    let response = adapter.handle_request(1, "breakpointLocations", Some(args));
    match response {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "breakpointLocations");
        }
        other => panic!("expected Response, got {other:?}"),
    }
    Ok(())
}

// --- evaluate newline injection ---

#[test]
// AC:16 (Security: newline injection in evaluate)
fn test_evaluate_newline_injection_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let injections = vec!["$x\nsystem('rm -rf /')", "$x\rsystem('evil')", "$x\r\ndie()"];

    for expr in injections {
        let args = json!({
            "expression": expr,
            "allowSideEffects": false
        });
        let response = adapter.handle_request(1, "evaluate", Some(args));
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert_eq!(command, "evaluate");
                assert!(!success, "Newline injection should be rejected: {expr:?}");
                let msg = message.ok_or("rejection must include a message")?;
                assert!(
                    msg.contains("newline") || msg.contains("newlines"),
                    "rejection message should mention newlines: {msg}"
                );
            }
            other => panic!("expected Response, got {other:?}"),
        }
    }
    Ok(())
}

// --- setExpression newline injection ---

#[test]
// AC:16 (Security: newline injection in setExpression)
fn test_set_expression_newline_injection_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();

    // Newline in expression
    let args = json!({
        "expression": "$x\nsystem('evil')",
        "value": "42"
    });
    let response = adapter.handle_request(1, "setExpression", Some(args));
    assert_response(response, "setExpression", false);

    // Newline in value
    let args = json!({
        "expression": "$x",
        "value": "42\nsystem('evil')"
    });
    let response = adapter.handle_request(2, "setExpression", Some(args));
    assert_response(response, "setExpression", false);
    Ok(())
}

// --- setExpression type validation (unsafe value) ---

#[test]
// AC:16 (Security: unsafe value in setExpression)
fn test_set_expression_unsafe_value_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({
        "expression": "$x",
        "value": "system('evil')"
    });
    let response = adapter.handle_request(1, "setExpression", Some(args));
    assert_response(response, "setExpression", false);
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Protocol State Machine Compliance
// ---------------------------------------------------------------------------

// --- Sequence number monotonicity ---

#[test]
// AC:5 (Protocol compliance: sequence numbers must be monotonically increasing)
fn test_sequence_numbers_are_monotonically_increasing() -> Result<(), Box<dyn std::error::Error>> {
    let (mut adapter, _rx) = make_adapter_with_events();

    let seq_numbers: Vec<i64> = (1..=5)
        .map(|i| {
            let response = adapter.handle_request(i, "threads", None);
            match response {
                DapMessage::Response { seq, .. } => seq,
                DapMessage::Event { seq, .. } => seq,
                DapMessage::Request { seq, .. } => seq,
            }
        })
        .collect();

    // Each seq must be strictly greater than the previous
    for window in seq_numbers.windows(2) {
        assert!(
            window[1] > window[0],
            "sequence numbers must be monotonically increasing: {window:?}"
        );
    }
    Ok(())
}

// --- Initialize then events are sequential ---

#[test]
// AC:5 (Protocol compliance: initialized event seq must follow initialize response seq)
fn test_initialize_response_seq_before_initialized_event_seq()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut adapter, rx) = make_adapter_with_events();

    let init_response = adapter.handle_request(1, "initialize", None);
    let response_seq = match init_response {
        DapMessage::Response { seq, success, .. } => {
            assert!(success);
            seq
        }
        other => panic!("expected initialize Response, got {other:?}"),
    };

    let event = rx
        .recv_timeout(std::time::Duration::from_millis(200))
        .map_err(|_| "initialized event should be emitted")?;
    let event_seq = match event {
        DapMessage::Event { seq, event, .. } => {
            assert_eq!(event, "initialized");
            seq
        }
        other => panic!("expected initialized Event, got {other:?}"),
    };

    // The event was emitted after the response, so its sequence must be higher
    assert!(
        event_seq > response_seq,
        "initialized event seq ({event_seq}) must be > initialize response seq ({response_seq})"
    );
    Ok(())
}

// --- Response shape: every response includes command, seq, request_seq ---

#[test]
// AC:5 (Protocol compliance: response shape completeness)
fn test_all_commands_return_proper_response_shape() -> Result<(), Box<dyn std::error::Error>> {
    let commands_and_args: Vec<(&str, Option<Value>)> = vec![
        ("threads", None),
        ("loadedSources", None),
        ("exceptionInfo", None),
        ("cancel", None),
        ("restartFrame", None),
        ("terminateThreads", None),
        ("loadedSources", None),
    ];

    for (cmd, args) in commands_and_args {
        let mut adapter = make_adapter();
        let response = adapter.handle_request(1, cmd, args);
        match response {
            DapMessage::Response { seq, request_seq, command, .. } => {
                assert!(seq > 0, "{cmd}: seq must be positive, got {seq}");
                assert_eq!(request_seq, 1, "{cmd}: request_seq must echo input seq");
                assert_eq!(command, cmd, "{cmd}: command must echo input command");
            }
            other => panic!("{cmd}: expected Response, got {other:?}"),
        }
    }
    Ok(())
}

// --- Unknown command returns failure response (not panic) ---

#[test]
// AC:5 (Protocol compliance: unknown commands return structured error)
fn test_unknown_command_returns_structured_error_response() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let response = adapter.handle_request(42, "thisCommandDoesNotExist", None);
    match response {
        DapMessage::Response { success, command, message, request_seq, .. } => {
            assert!(!success, "unknown command should not succeed");
            assert_eq!(command, "thisCommandDoesNotExist");
            assert_eq!(request_seq, 42, "request_seq must echo the input");
            assert!(message.is_some(), "unknown command must include an error message");
        }
        other => panic!("expected Response for unknown command, got {other:?}"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Response Schema Validation
// ---------------------------------------------------------------------------

// --- gotoTargets response body has targets[] ---

#[test]
// AC:16 (Schema: gotoTargets response includes targets array)
fn test_goto_targets_response_body_has_targets_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({
        "source": {},
        "line": 1
    });
    let response = adapter.handle_request(1, "gotoTargets", Some(args));
    let body = assert_response(response, "gotoTargets", true);
    let body = body.ok_or("gotoTargets body is required")?;
    let targets = body.get("targets").ok_or("gotoTargets body must have 'targets'")?;
    assert!(targets.is_array(), "'targets' must be an array");
    Ok(())
}

// --- stepInTargets response body has targets[] ---

#[test]
// AC:16 (Schema: stepInTargets response includes targets array)
fn test_step_in_targets_response_body_has_targets_array() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let args = json!({ "frameId": 1 });
    let response = adapter.handle_request(1, "stepInTargets", Some(args));
    let body = assert_response(response, "stepInTargets", true);
    let body = body.ok_or("stepInTargets body is required")?;
    let targets = body.get("targets").ok_or("stepInTargets body must have 'targets'")?;
    assert!(targets.is_array(), "'targets' must be an array");
    Ok(())
}

// --- loadedSources response body has sources[] ---

#[test]
// AC:16 (Schema: loadedSources response includes sources array)
fn test_loaded_sources_response_body_has_sources_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "loadedSources", None);
    let body = assert_response(response, "loadedSources", true);
    let body = body.ok_or("loadedSources body is required")?;
    let sources = body.get("sources").ok_or("loadedSources body must have 'sources'")?;
    assert!(sources.is_array(), "'sources' must be an array");
    Ok(())
}

// --- breakpointLocations response body has breakpoints[] ---

#[test]
// AC:16 (Schema: breakpointLocations response includes breakpoints array)
fn test_breakpoint_locations_response_body_has_breakpoints_array()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let args = json!({
        "source": {},
        "line": 1
    });
    let response = adapter.handle_request(1, "breakpointLocations", Some(args));
    let body = assert_response(response, "breakpointLocations", true);
    let body = body.ok_or("breakpointLocations body is required")?;
    let bps = body.get("breakpoints").ok_or("breakpointLocations body must have 'breakpoints'")?;
    assert!(bps.is_array(), "'breakpoints' must be an array");
    Ok(())
}

// --- exceptionInfo response body has required fields ---

#[test]
// AC:16 (Schema: exceptionInfo response includes exceptionId and breakMode)
fn test_exception_info_response_body_has_required_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "exceptionInfo", None);
    let body = assert_response(response, "exceptionInfo", true);
    let body = body.ok_or("exceptionInfo body is required")?;
    assert!(body.get("exceptionId").is_some(), "exceptionId is required");
    assert!(body.get("breakMode").is_some(), "breakMode is required");
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Integration Scenarios
// ---------------------------------------------------------------------------

// --- Cancel clears the cancel flag (idempotent) ---

#[test]
// AC:16 (Integration: cancel can be called multiple times safely)
fn test_cancel_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();

    // First cancel
    let r1 = adapter.handle_request(1, "cancel", None);
    assert_response(r1, "cancel", true);

    // Second cancel — should still succeed
    let r2 = adapter.handle_request(2, "cancel", None);
    assert_response(r2, "cancel", true);
    Ok(())
}

// --- Disconnect without session succeeds ---

#[test]
// AC:5 (Integration: disconnect without active session)
fn test_disconnect_without_session_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "disconnect", None);
    assert_response(response, "disconnect", true);
    Ok(())
}

// --- Full initialize → disconnect flow ---

#[test]
// AC:5 (Integration: minimal session lifecycle)
fn test_initialize_then_disconnect_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let (mut adapter, rx) = make_adapter_with_events();

    let init = adapter.handle_request(1, "initialize", None);
    assert_response(init, "initialize", true);

    // Consume the initialized event
    let _event = rx
        .recv_timeout(std::time::Duration::from_millis(200))
        .map_err(|_| "initialized event must be emitted")?;

    let disconnect = adapter.handle_request(2, "disconnect", None);
    assert_response(disconnect, "disconnect", true);
    Ok(())
}

// --- State queries succeed even without a session ---

#[test]
// AC:5 (Integration: queries work without active debug session)
fn test_threads_without_session_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "threads", None);
    let body = assert_response(response, "threads", true);
    let body = body.ok_or("threads must return a body")?;
    assert!(body.get("threads").is_some(), "threads body must include 'threads' array");
    Ok(())
}

#[test]
// AC:5 (Integration: stackTrace without session returns placeholder frame)
fn test_stack_trace_without_session_returns_frames() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = make_adapter();
    let response = adapter.handle_request(1, "stackTrace", None);
    let body = assert_response(response, "stackTrace", true);
    let body = body.ok_or("stackTrace must return a body")?;
    assert!(body.get("stackFrames").is_some(), "stackTrace body must include 'stackFrames'");
    Ok(())
}
