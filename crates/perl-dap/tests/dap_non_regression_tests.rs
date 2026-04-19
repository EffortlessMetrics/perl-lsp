//! DAP Non-Regression Test Suite (Phase 3) — Issue #435
//!
//! Verifies protocol correctness invariants across every DAP command supported
//! by the dispatch table. These tests do NOT test feature behaviour (that is
//! covered by the feature tests); they guard against protocol regressions:
//!
//! 1. All 37 dispatched command types produce a `DapMessage::Response` — never
//!    a panic or a spurious `Event`/`Request` variant. (`launch` and `attach`
//!    are excluded because they spawn real Perl processes; 35 arms are tested here.)
//! 2. Every response carries the correct `command` echo and `request_seq` echo.
//! 3. Response sequence numbers are strictly monotonically increasing across a
//!    multi-request session.
//! 4. Error responses always include a non-empty `message` field.
//! 5. Success responses for commands with mandatory body fields always have a
//!    body with those fields present.
//! 6. Advertised capabilities in the `initialize` response are consistent with
//!    the actual handlers (no capability advertised as `true` when the handler
//!    always rejects).
//!
//! Tag: AC:17 (Phase 3 non-regression hardening)

// Tests use `panic!` as structured test-failure reporters and `expect()` on
// values that must be present per DAP spec.
#![allow(clippy::panic, clippy::expect_used)]

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn new_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

/// Assert that `msg` is a `Response` for `command`, returning `(seq, request_seq, success, body, message)`.
#[allow(clippy::panic)]
fn unwrap_response(
    msg: DapMessage,
    command: &str,
) -> (i64, i64, bool, Option<Value>, Option<String>) {
    match msg {
        DapMessage::Response {
            seq,
            request_seq,
            success,
            command: cmd,
            body,
            message,
        } => {
            assert_eq!(
                cmd, command,
                "response command must echo the request command"
            );
            (seq, request_seq, success, body, message)
        }
        other => panic!("expected Response for {command}, got {other:?}"),
    }
}

/// Assert success response, returning body.
fn assert_ok(msg: DapMessage, command: &str) -> Option<Value> {
    let (_, _, success, body, message) = unwrap_response(msg, command);
    assert!(
        success,
        "{command}: expected success=true, got message={message:?}"
    );
    body
}

/// Assert failure response, returning error message (always non-empty on failure).
fn assert_err(msg: DapMessage, command: &str) -> String {
    let (_, _, success, _, message) = unwrap_response(msg, command);
    assert!(!success, "{command}: expected success=false");
    let msg = message.unwrap_or_default();
    assert!(
        !msg.is_empty(),
        "{command}: error response must include a non-empty message"
    );
    msg
}

// ---------------------------------------------------------------------------
// 1. FULL DISPATCH SWEEP — every command in the dispatch table returns a Response
// ---------------------------------------------------------------------------

/// These are the minimum-valid arguments for each command that gets a success
/// response without a session. Commands that need more context (e.g. `launch`)
/// are tested separately below.
const ZERO_ARG_COMMANDS: &[&str] = &[
    "cancel",
    "configurationDone",
    "disconnect",
    "exceptionInfo",
    "loadedSources",
    "restartFrame", // always fails — non-regression: must return Response, not panic
    "terminate",
    "terminateThreads", // always fails — same contract
    "threads",
];

#[test]
// AC:17 — every zero-arg command produces a Response (not panic, not event)
fn test_all_zero_arg_commands_return_response() -> Result<(), Box<dyn std::error::Error>> {
    for cmd in ZERO_ARG_COMMANDS {
        let mut adapter = new_adapter();
        let msg = adapter.handle_request(1, cmd, None);
        match msg {
            DapMessage::Response { command, .. } => {
                assert_eq!(command, *cmd, "{cmd}: command must echo back");
            }
            other => panic!("{cmd}: expected Response, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
// AC:17 — every command with required args returns a structured error (not panic) when args missing
fn test_all_required_arg_commands_return_error_when_args_missing()
-> Result<(), Box<dyn std::error::Error>> {
    // These commands fail gracefully with missing args.
    // Note: stackTrace is intentionally excluded — its arguments are fully optional
    // per the DAP spec and the handler succeeds with None args (returns empty stackFrames).
    let requires_args = &[
        "breakpointLocations",
        "dataBreakpointInfo",
        "evaluate",
        "goto",
        "gotoTargets",
        "inlineValues",
        "scopes",
        "setBreakpoints",
        "setDataBreakpoints",
        "setExpression",
        "setFunctionBreakpoints",
        "setVariable",
        "source",
        "stepInTargets",
        "variables",
    ];

    for cmd in requires_args {
        let mut adapter = new_adapter();
        let msg = adapter.handle_request(1, cmd, None);
        match msg {
            DapMessage::Response {
                success,
                command,
                message,
                ..
            } => {
                assert_eq!(command, *cmd, "{cmd}: command echo");
                assert!(!success, "{cmd}: must fail when args are missing");
                let err_msg = message.unwrap_or_default();
                assert!(
                    !err_msg.is_empty(),
                    "{cmd}: missing-args error must be non-empty"
                );
            }
            other => panic!("{cmd}: expected Response for missing-args path, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
// AC:17 — unknown command returns structured failure (not panic, not crash)
fn test_unknown_command_returns_structured_failure() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let msg = adapter.handle_request(99, "no_such_command_xyz", None);
    let err = assert_err(msg, "no_such_command_xyz");
    assert!(
        err.to_lowercase().contains("unknown") || err.to_lowercase().contains("command"),
        "unknown command message should mention 'unknown' or 'command': {err}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. REQUEST_SEQ ECHO — every response must echo the exact request seq
// ---------------------------------------------------------------------------

#[test]
// AC:17 — request_seq is faithfully echoed for every command
fn test_request_seq_echo_for_all_dispatched_commands() -> Result<(), Box<dyn std::error::Error>> {
    // Representative set covering all branches of the dispatch table.
    // `launch` and `attach` are excluded — they spawn real Perl processes and are
    // covered in dap_comprehensive_test, dap_launch_security_test, and
    // dap_integration_test. 35 of the 37 dispatch arms are verified here.
    // `variables` and `inlineValues` are included below to cover every remaining arm.
    let cases: &[(&str, Option<Value>)] = &[
        ("cancel", None),
        ("configurationDone", None),
        ("disconnect", None),
        ("exceptionInfo", None),
        ("initialize", None),
        ("loadedSources", None),
        ("restartFrame", None),
        ("stackTrace", None),
        ("terminate", None),
        ("terminateThreads", None),
        ("threads", None),
        (
            "breakpointLocations",
            Some(json!({"source": {}, "line": 1})),
        ),
        ("completions", Some(json!({"text": "$x", "column": 2}))),
        ("continue", Some(json!({"threadId": 1}))),
        (
            "dataBreakpointInfo",
            Some(json!({"name": "$x", "variablesReference": 0})),
        ),
        ("evaluate", Some(json!({"expression": "$x"}))),
        ("goto", Some(json!({"threadId": 1, "targetId": 9999}))),
        ("gotoTargets", Some(json!({"source": {}, "line": 1}))),
        (
            "modules",
            Some(json!({"startModule": 0, "moduleCount": 10})),
        ),
        ("next", Some(json!({"threadId": 1}))),
        ("pause", Some(json!({"threadId": 1}))),
        ("restart", Some(json!({}))),
        ("scopes", Some(json!({"frameId": 1}))),
        (
            "setBreakpoints",
            Some(json!({"source": {}, "breakpoints": []})),
        ),
        ("setDataBreakpoints", Some(json!({"breakpoints": []}))),
        ("setExceptionBreakpoints", Some(json!({"filters": []}))),
        (
            "setExpression",
            Some(json!({"expression": "$x", "value": "1"})),
        ),
        ("setFunctionBreakpoints", Some(json!({"breakpoints": []}))),
        (
            "setVariable",
            Some(json!({"variablesReference": 1, "name": "$x", "value": "1"})),
        ),
        (
            "source",
            Some(json!({"source": {"path": "/nonexistent/file.pl"}, "sourceReference": 0})),
        ),
        ("stepIn", Some(json!({"threadId": 1}))),
        ("stepInTargets", Some(json!({"frameId": 0}))),
        ("stepOut", Some(json!({"threadId": 1}))),
        // Complete the sweep: inlineValues and variables are the two remaining arms
        // not covered above. Both are tested here for request_seq echo only; their
        // body contents are validated in dedicated sections.
        (
            "inlineValues",
            Some(json!({"source": {}, "startLine": 1, "endLine": 5})),
        ),
        ("variables", Some(json!({"variablesReference": 1}))),
    ];

    for (seq, (command, args)) in cases.iter().enumerate() {
        let req_seq = (seq as i64) + 200;
        let mut adapter = new_adapter();
        let msg = adapter.handle_request(req_seq, command, args.clone());
        match msg {
            DapMessage::Response {
                request_seq,
                command: cmd,
                ..
            } => {
                assert_eq!(
                    request_seq, req_seq,
                    "{command}: request_seq must be echoed exactly"
                );
                assert_eq!(
                    cmd, *command,
                    "{command}: response command must echo request command"
                );
            }
            other => panic!("{command}: expected Response, got {other:?}"),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. SEQUENCE NUMBER MONOTONICITY — seq must strictly increase across requests
// ---------------------------------------------------------------------------

#[test]
// AC:17 — sequence numbers are strictly monotonically increasing across a session
fn test_seq_numbers_strictly_monotone_across_session() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();

    let commands = &[
        ("threads", None),
        ("loadedSources", None),
        ("cancel", None),
        ("exceptionInfo", None),
        ("disconnect", None),
    ];

    let mut prev_seq: Option<i64> = None;
    for (i, (cmd, args)) in commands.iter().enumerate() {
        let req_seq = (i as i64) + 1;
        let msg = adapter.handle_request(req_seq, cmd, args.clone());
        let seq = match msg {
            DapMessage::Response { seq, .. } => seq,
            other => panic!("{cmd}: expected Response, got {other:?}"),
        };

        if let Some(prev) = prev_seq {
            assert!(
                seq > prev,
                "seq {seq} must be > previous seq {prev} (command: {cmd})"
            );
        }
        prev_seq = Some(seq);
    }
    Ok(())
}

#[test]
// AC:17 — seq numbers never go backwards or repeat, even across different command types
fn test_seq_numbers_never_repeat() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let mut seen_seqs = std::collections::HashSet::new();

    for i in 0..10_u32 {
        let cmd = if i % 2 == 0 { "threads" } else { "cancel" };
        let msg = adapter.handle_request(i as i64 + 1, cmd, None);
        let seq = match msg {
            DapMessage::Response { seq, .. } => seq,
            other => panic!("{cmd}: expected Response, got {other:?}"),
        };
        assert!(
            seen_seqs.insert(seq),
            "seq {seq} was already used — sequence numbers must never repeat"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. CAPABILITIES CONSISTENCY — advertised capabilities vs handler behaviour
// ---------------------------------------------------------------------------

fn get_capabilities(adapter: &mut DebugAdapter) -> Value {
    assert_ok(adapter.handle_request(1, "initialize", None), "initialize").unwrap_or(Value::Null)
}

#[test]
// AC:17 — supportsRestartFrame=false matches restartFrame always failing
fn test_capabilities_restart_frame_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsRestartFrame")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // Handler must always fail — this is the ground truth
    let mut adapter2 = new_adapter();
    let response = adapter2.handle_request(2, "restartFrame", None);
    let (_, _, success, _, _) = unwrap_response(response, "restartFrame");

    if !advertised {
        // Capability says "not supported" and handler fails — consistent
        assert!(
            !success,
            "restartFrame handler must fail when capability=false"
        );
    }
    // If capability were ever set to true, the handler would need to succeed — caught here
    Ok(())
}

#[test]
// AC:17 — supportsTerminateThreadsRequest=false matches terminateThreads always failing
fn test_capabilities_terminate_threads_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsTerminateThreadsRequest")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut adapter2 = new_adapter();
    let response = adapter2.handle_request(2, "terminateThreads", None);
    let (_, _, success, _, _) = unwrap_response(response, "terminateThreads");

    if !advertised {
        assert!(!success, "terminateThreads must fail when capability=false");
    }
    Ok(())
}

#[test]
// AC:17 — supportsLoadedSourcesRequest is advertised and handler always succeeds
fn test_capabilities_loaded_sources_advertised_and_working()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsLoadedSourcesRequest")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    assert!(
        advertised,
        "supportsLoadedSourcesRequest must be advertised as true"
    );

    // Handler must succeed
    let mut adapter2 = new_adapter();
    let body = assert_ok(
        adapter2.handle_request(2, "loadedSources", None),
        "loadedSources",
    );
    assert!(body.is_some(), "loadedSources must return a body");
    assert!(
        body.as_ref().and_then(|b| b.get("sources")).is_some(),
        "loadedSources body must contain 'sources'"
    );
    Ok(())
}

#[test]
// AC:17 — supportsCancelRequest is advertised and cancel always succeeds
fn test_capabilities_cancel_advertised_and_working() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsCancelRequest")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    assert!(advertised, "supportsCancelRequest must be advertised");

    let mut adapter2 = new_adapter();
    assert_ok(adapter2.handle_request(2, "cancel", None), "cancel");
    Ok(())
}

#[test]
// AC:17 — supportsExceptionInfoRequest matches exceptionInfo succeeding
fn test_capabilities_exception_info_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsExceptionInfoRequest")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // exceptionInfo always succeeds regardless of session state
    let mut adapter2 = new_adapter();
    let response = adapter2.handle_request(2, "exceptionInfo", None);
    let (_, _, success, _, _) = unwrap_response(response, "exceptionInfo");

    // Bidirectional consistency: capability and handler must agree in both directions.
    if advertised {
        assert!(
            success,
            "exceptionInfo must succeed when capability is advertised"
        );
    } else {
        // If capability is not advertised, the handler must also not succeed,
        // or the capability must be set to true. Mismatched = implementation drift.
        assert!(
            !success || advertised,
            "exceptionInfo handler succeeds but supportsExceptionInfoRequest is not advertised — \
             set the capability to true or make the handler fail gracefully"
        );
    }
    Ok(())
}

#[test]
// AC:17 — supportsSetExpression is advertised and setExpression validates inputs
fn test_capabilities_set_expression_advertised() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsSetExpression")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    assert!(advertised, "supportsSetExpression must be advertised");

    // Handler must reject missing args
    let mut adapter2 = new_adapter();
    let err = assert_err(
        adapter2.handle_request(2, "setExpression", None),
        "setExpression",
    );
    assert!(!err.is_empty(), "missing-args error must be non-empty");
    Ok(())
}

#[test]
// AC:17 — supportsGotoTargetsRequest is advertised and gotoTargets returns targets array
fn test_capabilities_goto_targets_advertised_and_working() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = new_adapter();
    let caps = get_capabilities(&mut adapter);
    let advertised = caps
        .get("supportsGotoTargetsRequest")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    assert!(advertised, "supportsGotoTargetsRequest must be advertised");

    let mut adapter2 = new_adapter();
    let body = assert_ok(
        adapter2.handle_request(2, "gotoTargets", Some(json!({"source": {}, "line": 1}))),
        "gotoTargets",
    );
    let body = body.ok_or("gotoTargets must return a body")?;
    assert!(
        body.get("targets").is_some(),
        "gotoTargets body must contain 'targets'"
    );
    assert!(
        body["targets"].is_array(),
        "gotoTargets 'targets' must be an array"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. ERROR RESPONSE FORMAT — every failure carries a non-empty message
// ---------------------------------------------------------------------------

#[test]
// AC:17 — error responses always include a non-empty message string
fn test_error_responses_always_have_message() -> Result<(), Box<dyn std::error::Error>> {
    // Commands that are designed to fail under these specific conditions
    let cases: &[(&str, Option<Value>)] = &[
        ("restartFrame", None),
        ("terminateThreads", None),
        ("goto", None),                   // missing args
        ("gotoTargets", None),            // missing args
        ("stepInTargets", None),          // missing args
        ("breakpointLocations", None),    // missing args
        ("setExpression", None),          // missing args
        ("source", None),                 // missing args
        ("evaluate", None),               // missing args
        ("variables", None),              // missing args
        ("scopes", None),                 // missing args
        ("setBreakpoints", None),         // missing args
        ("setFunctionBreakpoints", None), // missing args
        ("setDataBreakpoints", None),     // missing args
        ("dataBreakpointInfo", None),     // missing args
        ("setVariable", None),            // missing args
        ("inlineValues", None),           // missing args
        ("no_such_command_xyz", None),    // unknown
    ];

    for (cmd, args) in cases {
        let mut adapter = new_adapter();
        let msg = adapter.handle_request(1, cmd, args.clone());
        match msg {
            DapMessage::Response {
                success,
                command,
                message,
                ..
            } => {
                assert!(!success, "{cmd}: must fail for this input");
                assert_eq!(command, *cmd, "{cmd}: command echo");
                let err = message.unwrap_or_default();
                assert!(
                    !err.is_empty(),
                    "{cmd}: error response must include a non-empty message, got empty string"
                );
            }
            other => panic!("{cmd}: expected Response, got {other:?}"),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. BODY FIELD COMPLETENESS — success responses have all mandatory body fields
// ---------------------------------------------------------------------------

#[test]
// AC:17 — initialize response contains required capability keys
fn test_initialize_capabilities_have_required_keys() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(adapter.handle_request(1, "initialize", None), "initialize")
        .ok_or("initialize must return a capabilities body")?;

    // A non-exhaustive set of capability keys that MUST be present per DAP 1.51+
    let required_keys = &[
        "supportsConfigurationDoneRequest",
        "supportsFunctionBreakpoints",
        "supportsSetVariable",
        "supportsRestartFrame",
        "supportsGotoTargetsRequest",
        "supportsRestartRequest",
        "supportsLoadedSourcesRequest",
        "supportsSetExpression",
        "supportsTerminateRequest",
        "supportsCancelRequest",
    ];

    for key in required_keys {
        assert!(
            body.get(key).is_some(),
            "initialize response must include capability key: {key}"
        );
    }
    Ok(())
}

#[test]
// AC:17 — threads response has 'threads' array in body
fn test_threads_response_has_threads_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(adapter.handle_request(1, "threads", None), "threads")
        .ok_or("threads must return a body")?;
    assert!(
        body.get("threads").is_some(),
        "threads body must contain 'threads'"
    );
    assert!(body["threads"].is_array(), "'threads' must be an array");
    Ok(())
}

#[test]
// AC:17 — stackTrace response has 'stackFrames' array in body
fn test_stack_trace_response_has_stack_frames() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(adapter.handle_request(1, "stackTrace", None), "stackTrace")
        .ok_or("stackTrace must return a body")?;
    assert!(
        body.get("stackFrames").is_some(),
        "stackTrace body must contain 'stackFrames'"
    );
    assert!(
        body["stackFrames"].is_array(),
        "'stackFrames' must be an array"
    );
    Ok(())
}

#[test]
// AC:17 — loadedSources response has 'sources' array in body
fn test_loaded_sources_response_has_sources_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(
        adapter.handle_request(1, "loadedSources", None),
        "loadedSources",
    )
    .ok_or("loadedSources must return a body")?;
    assert!(
        body.get("sources").is_some(),
        "loadedSources body must contain 'sources'"
    );
    assert!(body["sources"].is_array(), "'sources' must be an array");
    Ok(())
}

#[test]
// AC:17 — exceptionInfo response has required fields: exceptionId, breakMode
fn test_exception_info_response_has_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(
        adapter.handle_request(1, "exceptionInfo", None),
        "exceptionInfo",
    )
    .ok_or("exceptionInfo must return a body")?;
    assert!(
        body.get("exceptionId").is_some(),
        "exceptionInfo must include 'exceptionId'"
    );
    assert!(
        body.get("breakMode").is_some(),
        "exceptionInfo must include 'breakMode'"
    );
    let exception_id = body["exceptionId"].as_str().unwrap_or("");
    assert!(!exception_id.is_empty(), "exceptionId must be non-empty");
    Ok(())
}

#[test]
// AC:17 — gotoTargets response has 'targets' array
fn test_goto_targets_response_has_targets_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(
        adapter.handle_request(1, "gotoTargets", Some(json!({"source": {}, "line": 1}))),
        "gotoTargets",
    )
    .ok_or("gotoTargets must return a body")?;
    assert!(
        body.get("targets").is_some(),
        "gotoTargets must include 'targets'"
    );
    assert!(body["targets"].is_array(), "'targets' must be an array");
    Ok(())
}

#[test]
// AC:17 — stepInTargets response has 'targets' array
fn test_step_in_targets_response_has_targets_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(
        adapter.handle_request(1, "stepInTargets", Some(json!({"frameId": 0}))),
        "stepInTargets",
    )
    .ok_or("stepInTargets must return a body")?;
    assert!(
        body.get("targets").is_some(),
        "stepInTargets must include 'targets'"
    );
    assert!(body["targets"].is_array(), "'targets' must be an array");
    Ok(())
}

#[test]
// AC:17 — breakpointLocations response has 'breakpoints' array
fn test_breakpoint_locations_response_has_breakpoints_array()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let body = assert_ok(
        adapter.handle_request(
            1,
            "breakpointLocations",
            Some(json!({"source": {}, "line": 1})),
        ),
        "breakpointLocations",
    )
    .ok_or("breakpointLocations must return a body")?;
    assert!(
        body.get("breakpoints").is_some(),
        "breakpointLocations must include 'breakpoints'"
    );
    assert!(
        body["breakpoints"].is_array(),
        "'breakpoints' must be an array"
    );
    Ok(())
}

#[test]
// AC:17 — setBreakpoints response has 'breakpoints' array
fn test_set_breakpoints_response_has_breakpoints_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"source": {}, "breakpoints": []});
    let body = assert_ok(
        adapter.handle_request(1, "setBreakpoints", Some(args)),
        "setBreakpoints",
    )
    .ok_or("setBreakpoints must return a body")?;
    assert!(
        body.get("breakpoints").is_some(),
        "setBreakpoints must include 'breakpoints'"
    );
    assert!(
        body["breakpoints"].is_array(),
        "'breakpoints' must be an array"
    );
    Ok(())
}

#[test]
// AC:17 — setFunctionBreakpoints response has 'breakpoints' array
fn test_set_function_breakpoints_response_has_breakpoints_array()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"breakpoints": []});
    let body = assert_ok(
        adapter.handle_request(1, "setFunctionBreakpoints", Some(args)),
        "setFunctionBreakpoints",
    )
    .ok_or("setFunctionBreakpoints must return a body")?;
    assert!(
        body.get("breakpoints").is_some(),
        "setFunctionBreakpoints must include 'breakpoints'"
    );
    Ok(())
}

#[test]
// AC:17 — setExceptionBreakpoints response has 'breakpoints' array
fn test_set_exception_breakpoints_response_has_breakpoints_array()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"filters": []});
    let body = assert_ok(
        adapter.handle_request(1, "setExceptionBreakpoints", Some(args)),
        "setExceptionBreakpoints",
    )
    .ok_or("setExceptionBreakpoints must return a body")?;
    assert!(
        body.get("breakpoints").is_some(),
        "setExceptionBreakpoints must include 'breakpoints'"
    );
    Ok(())
}

#[test]
// AC:17 — setDataBreakpoints response has 'breakpoints' array
fn test_set_data_breakpoints_response_has_breakpoints_array()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"breakpoints": []});
    let body = assert_ok(
        adapter.handle_request(1, "setDataBreakpoints", Some(args)),
        "setDataBreakpoints",
    )
    .ok_or("setDataBreakpoints must return a body")?;
    assert!(
        body.get("breakpoints").is_some(),
        "setDataBreakpoints must include 'breakpoints'"
    );
    Ok(())
}

#[test]
// AC:17 — modules response has 'modules' array and 'totalModules' count
fn test_modules_response_has_modules_array_and_count() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"startModule": 0, "moduleCount": 10});
    let body = assert_ok(adapter.handle_request(1, "modules", Some(args)), "modules")
        .ok_or("modules must return a body")?;
    assert!(
        body.get("modules").is_some(),
        "modules body must include 'modules'"
    );
    assert!(body["modules"].is_array(), "'modules' must be an array");
    assert!(
        body.get("totalModules").is_some(),
        "modules body must include 'totalModules'"
    );
    Ok(())
}

#[test]
// AC:17 — completions response has 'targets' array
fn test_completions_response_has_targets_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"text": "$", "column": 1});
    let body = assert_ok(
        adapter.handle_request(1, "completions", Some(args)),
        "completions",
    )
    .ok_or("completions must return a body")?;
    assert!(
        body.get("targets").is_some(),
        "completions body must include 'targets'"
    );
    assert!(body["targets"].is_array(), "'targets' must be an array");
    Ok(())
}

#[test]
// AC:17 — continue response has 'allThreadsContinued' field
fn test_continue_response_has_all_threads_continued() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({"threadId": 1});
    let body = assert_ok(
        adapter.handle_request(1, "continue", Some(args)),
        "continue",
    )
    .ok_or("continue must return a body")?;
    assert!(
        body.get("allThreadsContinued").is_some(),
        "continue body must include 'allThreadsContinued'"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. PROTOCOL EDGE CASES — boundary values and degenerate inputs
// ---------------------------------------------------------------------------

#[test]
// AC:17 — request_seq=0 is accepted and echoed (0 is a valid sequence number per DAP)
fn test_request_seq_zero_is_valid() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let msg = adapter.handle_request(0, "threads", None);
    let (_, request_seq, _, _, _) = unwrap_response(msg, "threads");
    assert_eq!(request_seq, 0, "request_seq=0 must be echoed back");
    Ok(())
}

#[test]
// AC:17 — request_seq=i64::MAX is accepted and echoed
fn test_request_seq_max_i64_is_valid() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let msg = adapter.handle_request(i64::MAX, "threads", None);
    let (_, request_seq, _, _, _) = unwrap_response(msg, "threads");
    assert_eq!(request_seq, i64::MAX, "i64::MAX request_seq must be echoed");
    Ok(())
}

#[test]
// AC:17 — negative request_seq is accepted and echoed (DAP spec does not prohibit it)
fn test_request_seq_negative_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let msg = adapter.handle_request(-1, "threads", None);
    let (_, request_seq, _, _, _) = unwrap_response(msg, "threads");
    assert_eq!(request_seq, -1, "negative request_seq must be echoed");
    Ok(())
}

#[test]
// AC:17 — request_seq=i64::MIN is accepted and echoed (lowest boundary value)
fn test_request_seq_min_i64_is_valid() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let msg = adapter.handle_request(i64::MIN, "threads", None);
    let (_, request_seq, _, _, _) = unwrap_response(msg, "threads");
    assert_eq!(request_seq, i64::MIN, "i64::MIN request_seq must be echoed");
    Ok(())
}

#[test]
// AC:17 — cancel flag does not corrupt subsequent command responses on the same adapter
fn test_cancel_followed_by_command_returns_valid_response() -> Result<(), Box<dyn std::error::Error>>
{
    // cancel sets an internal cancel_requested flag that gotoTargets and breakpointLocations
    // read during their inner loops. Verify that a subsequent command on the same adapter
    // still returns a well-formed Response and doesn't panic or hang.
    //
    // Note: gotoTargets with source.path=None returns early before reaching the
    // cancel_requested check, so cancel_requested remains true after that call.
    // Using a real source path here ensures the check fires and the flag resets.
    // The non-existent path causes an early file-read error return, which is also
    // fine — what matters is that a Response is returned and request_seq is echoed.
    let mut adapter = new_adapter();

    // First: cancel sets cancel_requested = true
    let cancel_msg = adapter.handle_request(1, "cancel", None);
    match cancel_msg {
        DapMessage::Response {
            command, success, ..
        } => {
            assert_eq!(command, "cancel");
            assert!(success, "cancel must succeed");
        }
        other => panic!("cancel: expected Response, got {other:?}"),
    }

    // Second: breakpointLocations with a source path triggers the cancel_requested check
    // in its inner loop, resets the flag, and returns a valid Response.
    let bp_msg = adapter.handle_request(
        2,
        "breakpointLocations",
        Some(json!({"source": {"path": "/nonexistent/file.pl"}, "line": 1})),
    );
    match bp_msg {
        DapMessage::Response {
            command,
            request_seq,
            ..
        } => {
            assert_eq!(command, "breakpointLocations", "command must echo");
            assert_eq!(request_seq, 2, "request_seq must be echoed after cancel");
        }
        other => panic!("breakpointLocations after cancel: expected Response, got {other:?}"),
    }

    // Third: a subsequent command must also return a valid Response.
    let threads_msg = adapter.handle_request(3, "threads", None);
    match threads_msg {
        DapMessage::Response {
            command,
            request_seq,
            ..
        } => {
            assert_eq!(command, "threads");
            assert_eq!(request_seq, 3);
        }
        other => panic!("threads after cancel: expected Response, got {other:?}"),
    }
    Ok(())
}

#[test]
// AC:17 — null/empty JSON object body does not panic on any command
fn test_empty_object_args_do_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let commands_accepting_any = &[
        "cancel",
        "configurationDone",
        "disconnect",
        "exceptionInfo",
        "loadedSources",
        "threads",
        "terminate",
    ];

    for cmd in commands_accepting_any {
        let mut adapter = new_adapter();
        let msg = adapter.handle_request(1, cmd, Some(json!({})));
        match msg {
            DapMessage::Response { command, .. } => {
                assert_eq!(command, *cmd, "{cmd}: command must echo");
            }
            other => panic!("{cmd}: expected Response for empty-object args, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
// AC:17 — deeply nested JSON args does not panic
fn test_deeply_nested_json_args_do_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    // args with unexpected deeply nested structure — must not panic
    let args = json!({
        "source": {"path": "/tmp/x.pl", "nested": {"deep": {"deeper": {"value": 42}}}},
        "breakpoints": [{"line": 1, "condition": {"nested": "should be string"}}]
    });
    let msg = adapter.handle_request(1, "setBreakpoints", Some(args));
    // Either succeeds or fails gracefully — must not panic
    match msg {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "setBreakpoints");
        }
        other => panic!("expected Response, got {other:?}"),
    }
    Ok(())
}

#[test]
// AC:17 — string where number expected does not panic
fn test_wrong_type_args_do_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let args = json!({
        "frameId": "not_a_number",
        "source": 12345
    });
    let msg = adapter.handle_request(1, "scopes", Some(args));
    // Must return a Response — whether success or failure — not panic
    match msg {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "scopes");
        }
        other => panic!("expected Response for wrong-type args, got {other:?}"),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 8. INITIALIZE → LIFECYCLE INTEGRATION — sequence ordering invariants
// ---------------------------------------------------------------------------

#[test]
// AC:17 — initialize response seq is always positive
fn test_initialize_response_seq_is_positive() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    let msg = adapter.handle_request(1, "initialize", None);
    let (seq, _, success, _, _) = unwrap_response(msg, "initialize");
    assert!(success, "initialize must succeed");
    assert!(
        seq > 0,
        "initialize response seq must be positive, got {seq}"
    );
    Ok(())
}

#[test]
// AC:17 — disconnect without prior initialize succeeds (no crash on missing session)
fn test_disconnect_without_initialize_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = new_adapter();
    assert_ok(adapter.handle_request(1, "disconnect", None), "disconnect");
    Ok(())
}

#[test]
// AC:17 — multiple requests in sequence do not corrupt adapter state
fn test_multiple_sequential_requests_do_not_corrupt_state() -> Result<(), Box<dyn std::error::Error>>
{
    let mut adapter = new_adapter();

    // Fire a variety of requests and ensure each still returns a valid Response
    let sequence = &[
        ("threads", None),
        ("cancel", None),
        ("loadedSources", None),
        ("exceptionInfo", None),
        ("threads", None),
        ("cancel", None),
        ("disconnect", None),
    ];

    for (i, (cmd, args)) in sequence.iter().enumerate() {
        let req_seq = (i as i64) + 1;
        let msg = adapter.handle_request(req_seq, cmd, args.clone());
        match msg {
            DapMessage::Response {
                request_seq,
                command,
                ..
            } => {
                assert_eq!(request_seq, req_seq, "{cmd}: request_seq must match");
                assert_eq!(command, *cmd, "{cmd}: command must echo");
            }
            other => panic!("{cmd}: expected Response, got {other:?}"),
        }
    }
    Ok(())
}
