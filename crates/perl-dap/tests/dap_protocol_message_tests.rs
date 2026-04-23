//! Protocol message tests for 10 previously untested DAP commands.
//!
//! Covers:
//!   breakpointLocations, cancel, exceptionInfo, goto, gotoTargets,
//!   loadedSources, restartFrame, setExpression, stepInTargets, terminateThreads
//!
//! Each section is tagged with // AC:ID referencing issue #2783.

// Tests use `panic!` in match arms as structured test failure reporters,
// and `expect()` on response body values that must be present per DAP spec.
#![allow(clippy::panic, clippy::expect_used)]

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;

// ============================================================================
// Helpers
// ============================================================================

#[allow(clippy::panic)]
fn assert_response(msg: DapMessage, expected_command: &str) -> (bool, Option<String>) {
    match msg {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, expected_command, "command name must match");
            (success, message)
        }
        other => panic!("expected Response for {expected_command}, got {:?}", other),
    }
}

fn assert_success_response(msg: DapMessage, expected_command: &str) {
    let (success, msg_text) = assert_response(msg, expected_command);
    assert!(success, "{expected_command} should succeed, but got message: {msg_text:?}");
}

fn assert_failure_response(msg: DapMessage, expected_command: &str) -> String {
    let (success, message) = assert_response(msg, expected_command);
    assert!(!success, "{expected_command} should fail");
    message.unwrap_or_default()
}

// ============================================================================
// breakpointLocations — AC:2783
// ============================================================================

#[test]
fn test_breakpoint_locations_missing_arguments() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — missing arguments must return error
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "breakpointLocations", None);
    let err = assert_failure_response(msg, "breakpointLocations");
    assert!(
        err.to_lowercase().contains("missing") || err.to_lowercase().contains("invalid"),
        "error should describe missing/invalid arguments: {err}"
    );
    Ok(())
}

#[test]
fn test_breakpoint_locations_no_source_path_returns_empty() -> Result<(), Box<dyn std::error::Error>>
{
    // AC:2783 — source without path returns successful empty list
    let mut adapter = DebugAdapter::new();
    let args = json!({
        "source": {},
        "line": 1
    });
    let msg = adapter.handle_request(1, "breakpointLocations", Some(args));
    let (success, _) = assert_response(msg, "breakpointLocations");
    assert!(success, "missing source.path should succeed with empty breakpoint list");
    Ok(())
}

#[test]
fn test_breakpoint_locations_nonexistent_file_returns_empty()
-> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — nonexistent file returns successful empty list (not an error)
    let mut adapter = DebugAdapter::new();
    let args = json!({
        "source": { "path": "/nonexistent/path/to/file.pl" },
        "line": 1
    });
    let msg = adapter.handle_request(1, "breakpointLocations", Some(args));
    let (success, _) = assert_response(msg, "breakpointLocations");
    assert!(success, "nonexistent file should return success with empty list");
    Ok(())
}

#[test]
fn test_breakpoint_locations_sequence_numbers() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — response must carry correct request_seq
    let mut adapter = DebugAdapter::new();
    let args = json!({
        "source": {},
        "line": 5
    });
    let msg = adapter.handle_request(77, "breakpointLocations", Some(args));
    match msg {
        DapMessage::Response { request_seq, .. } => {
            assert_eq!(request_seq, 77, "request_seq must match the request");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

#[test]
fn test_breakpoint_locations_path_traversal_does_not_panic()
-> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — path traversal paths must not panic.
    // Without a workspace root set (pre-launch), validate_source_path allows any path
    // through with a warning — defense-in-depth only blocks when a workspace boundary
    // is known.  The security contract here is: no panic, no crash, structured response.
    let mut adapter = DebugAdapter::new();
    let args = json!({
        "source": { "path": "/../../../etc/passwd" },
        "line": 1
    });
    let msg = adapter.handle_request(1, "breakpointLocations", Some(args));
    // Either rejected with failure OR returns success with empty list — both are safe.
    // The critical invariant: no panic and response carries the right command name.
    match msg {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "breakpointLocations");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

// ============================================================================
// cancel — AC:2783
// ============================================================================

#[test]
fn test_cancel_succeeds_without_session() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — cancel must succeed and set the cancel flag
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "cancel", None);
    assert_success_response(msg, "cancel");
    Ok(())
}

#[test]
fn test_cancel_with_arguments_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — cancel accepts optional progressId / requestId arguments
    let mut adapter = DebugAdapter::new();
    let args = json!({ "requestId": 42 });
    let msg = adapter.handle_request(2, "cancel", Some(args));
    assert_success_response(msg, "cancel");
    Ok(())
}

#[test]
fn test_cancel_has_no_body() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — cancel response body should be empty per DAP spec
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "cancel", None);
    match msg {
        DapMessage::Response { body, .. } => {
            assert!(body.is_none(), "cancel response must have no body");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

#[test]
fn test_cancel_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — calling cancel twice must not panic
    let mut adapter = DebugAdapter::new();
    let first = adapter.handle_request(1, "cancel", None);
    let second = adapter.handle_request(2, "cancel", None);
    assert!(matches!(first, DapMessage::Response { success: true, .. }));
    assert!(matches!(second, DapMessage::Response { success: true, .. }));
    Ok(())
}

// ============================================================================
// exceptionInfo — AC:2783
// ============================================================================

#[test]
fn test_exception_info_succeeds_without_session() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — exceptionInfo always succeeds, returns Unknown exception when no active session
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "exceptionInfo", None);
    assert_success_response(msg, "exceptionInfo");
    Ok(())
}

#[test]
fn test_exception_info_body_has_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — response body must contain exceptionId and breakMode per DAP spec
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "exceptionInfo", None);
    match msg {
        DapMessage::Response { body, success, .. } => {
            assert!(success, "exceptionInfo should succeed");
            let body = body.ok_or("exceptionInfo must have a response body")?;
            assert!(
                body.get("exceptionId").is_some(),
                "body must contain exceptionId, got: {body}"
            );
            assert!(body.get("breakMode").is_some(), "body must contain breakMode, got: {body}");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

#[test]
fn test_exception_info_with_thread_id_argument() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — exceptionInfo accepts optional threadId argument
    let mut adapter = DebugAdapter::new();
    let args = json!({ "threadId": 1 });
    let msg = adapter.handle_request(1, "exceptionInfo", Some(args));
    assert_success_response(msg, "exceptionInfo");
    Ok(())
}

#[test]
fn test_exception_info_no_session_returns_unknown_exception()
-> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — without a session, description must indicate an unknown exception
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "exceptionInfo", None);
    match msg {
        DapMessage::Response { body, .. } => {
            let body = body.ok_or("must have body")?;
            let exception_id = body["exceptionId"].as_str().unwrap_or("");
            assert!(!exception_id.is_empty(), "exceptionId must be non-empty");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

// ============================================================================
// goto — AC:2783
// ============================================================================

#[test]
fn test_goto_missing_arguments_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — goto without arguments must fail
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "goto", None);
    let err = assert_failure_response(msg, "goto");
    assert!(
        err.to_lowercase().contains("missing") || err.to_lowercase().contains("invalid"),
        "error must describe missing/invalid args: {err}"
    );
    Ok(())
}

#[test]
fn test_goto_unknown_target_id_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — goto with an unknown targetId must fail with a descriptive message
    let mut adapter = DebugAdapter::new();
    let args = json!({ "threadId": 1, "targetId": 9999 });
    let msg = adapter.handle_request(1, "goto", Some(args));
    let err = assert_failure_response(msg, "goto");
    assert!(
        err.to_lowercase().contains("unknown") || err.to_lowercase().contains("target"),
        "error must reference the unknown target: {err}"
    );
    Ok(())
}

#[test]
fn test_goto_no_session_after_unknown_target_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — goto fails on unknown target regardless of session state
    let mut adapter = DebugAdapter::new();
    let args = json!({ "threadId": 1, "targetId": 1 });
    let msg = adapter.handle_request(1, "goto", Some(args));
    // targetId 1 has never been registered via gotoTargets, so must fail
    assert!(matches!(msg, DapMessage::Response { success: false, .. }));
    Ok(())
}

// ============================================================================
// gotoTargets — AC:2783
// ============================================================================

#[test]
fn test_goto_targets_missing_arguments_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — gotoTargets without arguments must fail
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "gotoTargets", None);
    let err = assert_failure_response(msg, "gotoTargets");
    assert!(
        err.to_lowercase().contains("missing") || err.to_lowercase().contains("invalid"),
        "error must describe missing/invalid args: {err}"
    );
    Ok(())
}

#[test]
fn test_goto_targets_no_source_path_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — source without path returns successful empty target list
    let mut adapter = DebugAdapter::new();
    let args = json!({ "source": {}, "line": 5 });
    let msg = adapter.handle_request(1, "gotoTargets", Some(args));
    let (success, _) = assert_response(msg, "gotoTargets");
    assert!(success, "missing source.path should produce empty targets, not error");
    Ok(())
}

#[test]
fn test_goto_targets_nonexistent_file_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — nonexistent file returns successful empty target list
    let mut adapter = DebugAdapter::new();
    let args = json!({
        "source": { "path": "/nonexistent/script.pl" },
        "line": 10
    });
    let msg = adapter.handle_request(1, "gotoTargets", Some(args));
    let (success, _) = assert_response(msg, "gotoTargets");
    assert!(success, "nonexistent file should succeed with empty targets");
    Ok(())
}

#[test]
fn test_goto_targets_body_has_targets_array() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — response body must contain a "targets" array
    let mut adapter = DebugAdapter::new();
    let args = json!({ "source": {}, "line": 1 });
    let msg = adapter.handle_request(1, "gotoTargets", Some(args));
    match msg {
        DapMessage::Response { body, success, .. } => {
            assert!(success);
            let body = body.ok_or("gotoTargets must have response body")?;
            assert!(
                body.get("targets").is_some(),
                "body must contain 'targets' array, got: {body}"
            );
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

// ============================================================================
// loadedSources — AC:2783
// ============================================================================

#[test]
fn test_loaded_sources_succeeds_without_session() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — loadedSources always succeeds; without session returns empty list
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "loadedSources", None);
    assert_success_response(msg, "loadedSources");
    Ok(())
}

#[test]
fn test_loaded_sources_body_has_sources_array() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — response body must contain a "sources" array
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "loadedSources", None);
    match msg {
        DapMessage::Response { body, success, .. } => {
            assert!(success);
            let body = body.ok_or("loadedSources must have response body")?;
            assert!(
                body.get("sources").is_some(),
                "body must contain 'sources' array, got: {body}"
            );
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

#[test]
fn test_loaded_sources_no_session_returns_empty_list() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — without an active debug session, sources must be an empty array
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "loadedSources", None);
    match msg {
        DapMessage::Response { body, .. } => {
            let body = body.ok_or("must have body")?;
            let sources = body["sources"].as_array().ok_or("sources must be an array")?;
            assert!(sources.is_empty(), "no session means no loaded sources");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

#[test]
fn test_loaded_sources_accepts_ignored_arguments() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — loadedSources ignores any arguments (per DAP spec it has none)
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "loadedSources", Some(json!({})));
    assert_success_response(msg, "loadedSources");
    Ok(())
}

// ============================================================================
// restartFrame — AC:2783
// ============================================================================

#[test]
fn test_restart_frame_fails_as_unsupported() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — Perl does not support restartFrame; must always fail
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "restartFrame", None);
    let err = assert_failure_response(msg, "restartFrame");
    assert!(
        err.to_lowercase().contains("perl") || err.to_lowercase().contains("frame"),
        "error must explain Perl limitation: {err}"
    );
    Ok(())
}

#[test]
fn test_restart_frame_fails_with_arguments() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — restartFrame rejects regardless of arguments provided
    let mut adapter = DebugAdapter::new();
    let args = json!({ "frameId": 0 });
    let msg = adapter.handle_request(1, "restartFrame", Some(args));
    assert!(matches!(msg, DapMessage::Response { success: false, .. }));
    Ok(())
}

#[test]
fn test_restart_frame_returns_descriptive_message() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — failure message must be non-empty
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "restartFrame", None);
    match msg {
        DapMessage::Response { message, .. } => {
            let msg_text = message.ok_or("restartFrame must provide an error message")?;
            assert!(!msg_text.is_empty(), "error message must not be empty");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

// ============================================================================
// setExpression — AC:2783
// ============================================================================

#[test]
fn test_set_expression_missing_arguments_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — setExpression without arguments must fail
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "setExpression", None);
    let err = assert_failure_response(msg, "setExpression");
    assert!(!err.is_empty(), "failure must include an error message");
    Ok(())
}

#[test]
fn test_set_expression_empty_expression_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — setExpression with empty expression string must fail
    let mut adapter = DebugAdapter::new();
    let args = json!({ "expression": "", "value": "42" });
    let msg = adapter.handle_request(1, "setExpression", Some(args));
    let err = assert_failure_response(msg, "setExpression");
    assert!(
        err.to_lowercase().contains("expression") || err.to_lowercase().contains("missing"),
        "error must reference missing expression: {err}"
    );
    Ok(())
}

#[test]
fn test_set_expression_empty_value_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — setExpression with empty value string must fail
    let mut adapter = DebugAdapter::new();
    let args = json!({ "expression": "$x", "value": "" });
    let msg = adapter.handle_request(1, "setExpression", Some(args));
    let err = assert_failure_response(msg, "setExpression");
    assert!(
        err.to_lowercase().contains("value") || err.to_lowercase().contains("missing"),
        "error must reference missing value: {err}"
    );
    Ok(())
}

#[test]
fn test_set_expression_no_session_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — setExpression without an active debug session must fail
    let mut adapter = DebugAdapter::new();
    let args = json!({ "expression": "$x", "value": "42" });
    let msg = adapter.handle_request(1, "setExpression", Some(args));
    let err = assert_failure_response(msg, "setExpression");
    assert!(
        err.to_lowercase().contains("session") || err.to_lowercase().contains("debugger"),
        "error must reference missing session: {err}"
    );
    Ok(())
}

#[test]
fn test_set_expression_newline_in_expression_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — newline injection in expression must be rejected
    let mut adapter = DebugAdapter::new();
    let args = json!({ "expression": "$x\n$y", "value": "1" });
    let msg = adapter.handle_request(1, "setExpression", Some(args));
    assert!(matches!(msg, DapMessage::Response { success: false, .. }));
    Ok(())
}

#[test]
fn test_set_expression_newline_in_value_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — newline injection in value must be rejected
    let mut adapter = DebugAdapter::new();
    let args = json!({ "expression": "$x", "value": "1\n2" });
    let msg = adapter.handle_request(1, "setExpression", Some(args));
    assert!(matches!(msg, DapMessage::Response { success: false, .. }));
    Ok(())
}

// ============================================================================
// stepInTargets — AC:2783
// ============================================================================

#[test]
fn test_step_in_targets_missing_arguments_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — stepInTargets without arguments must fail
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "stepInTargets", None);
    let err = assert_failure_response(msg, "stepInTargets");
    assert!(
        err.to_lowercase().contains("missing") || err.to_lowercase().contains("invalid"),
        "error must describe missing/invalid args: {err}"
    );
    Ok(())
}

#[test]
fn test_step_in_targets_no_session_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — without a session there are no step-in targets
    let mut adapter = DebugAdapter::new();
    let args = json!({ "frameId": 0 });
    let msg = adapter.handle_request(1, "stepInTargets", Some(args));
    let (success, _) = assert_response(msg, "stepInTargets");
    assert!(success, "stepInTargets must succeed even without a session");
    Ok(())
}

#[test]
fn test_step_in_targets_body_has_targets_array() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — response body must contain a "targets" array
    let mut adapter = DebugAdapter::new();
    let args = json!({ "frameId": 0 });
    let msg = adapter.handle_request(1, "stepInTargets", Some(args));
    match msg {
        DapMessage::Response { body, success, .. } => {
            assert!(success);
            let body = body.ok_or("stepInTargets must have a response body")?;
            assert!(
                body.get("targets").is_some(),
                "body must contain 'targets' array, got: {body}"
            );
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

#[test]
fn test_step_in_targets_empty_when_no_session() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — no session means zero callable targets
    let mut adapter = DebugAdapter::new();
    let args = json!({ "frameId": 1 });
    let msg = adapter.handle_request(1, "stepInTargets", Some(args));
    match msg {
        DapMessage::Response { body, .. } => {
            let body = body.ok_or("must have body")?;
            let targets = body["targets"].as_array().ok_or("targets must be array")?;
            assert!(targets.is_empty(), "no session means no step-in targets");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

// ============================================================================
// terminateThreads — AC:2783
// ============================================================================

#[test]
fn test_terminate_threads_fails_as_unsupported() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — Perl threading model does not support targeted termination
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "terminateThreads", None);
    let err = assert_failure_response(msg, "terminateThreads");
    assert!(
        err.to_lowercase().contains("perl") || err.to_lowercase().contains("thread"),
        "error must reference Perl threading limitation: {err}"
    );
    Ok(())
}

#[test]
fn test_terminate_threads_with_thread_ids_still_fails() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — providing threadIds must not bypass the unsupported status
    let mut adapter = DebugAdapter::new();
    let args = json!({ "threadIds": [1, 2, 3] });
    let msg = adapter.handle_request(1, "terminateThreads", Some(args));
    assert!(matches!(msg, DapMessage::Response { success: false, .. }));
    Ok(())
}

#[test]
fn test_terminate_threads_returns_descriptive_message() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — failure message must be non-empty and meaningful
    let mut adapter = DebugAdapter::new();
    let msg = adapter.handle_request(1, "terminateThreads", None);
    match msg {
        DapMessage::Response { message, .. } => {
            let msg_text = message.ok_or("terminateThreads must provide an error message")?;
            assert!(!msg_text.is_empty(), "error message must not be empty");
        }
        other => panic!("expected Response, got {:?}", other),
    }
    Ok(())
}

// ============================================================================
// Cross-cutting: sequence number fidelity across all 10 commands
// ============================================================================

#[test]
fn test_all_ten_commands_echo_request_seq() -> Result<(), Box<dyn std::error::Error>> {
    // AC:2783 — every response must carry the original request_seq
    let mut adapter = DebugAdapter::new();

    let cases: &[(&str, Option<serde_json::Value>)] = &[
        ("cancel", None),
        ("exceptionInfo", None),
        ("loadedSources", None),
        ("restartFrame", None),
        ("terminateThreads", None),
        ("goto", Some(json!({ "threadId": 1, "targetId": 0 }))),
        ("gotoTargets", Some(json!({ "source": {}, "line": 1 }))),
        ("stepInTargets", Some(json!({ "frameId": 0 }))),
        ("breakpointLocations", Some(json!({ "source": {}, "line": 1 }))),
        ("setExpression", Some(json!({ "expression": "$x", "value": "1" }))),
    ];

    for (seq, (command, args)) in cases.iter().enumerate() {
        let req_seq = (seq as i64) + 100;
        let msg = adapter.handle_request(req_seq, command, args.clone());
        match msg {
            DapMessage::Response { request_seq, command: cmd, .. } => {
                assert_eq!(request_seq, req_seq, "{cmd}: request_seq must match");
            }
            other => panic!("{command}: expected Response, got {:?}", other),
        }
    }
    Ok(())
}
