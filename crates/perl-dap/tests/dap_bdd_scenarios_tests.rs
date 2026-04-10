//! Behavior-driven integration scenarios for `perl-dap`.
//!
//! These tests focus on end-user observable protocol behavior across multiple
//! request/response interactions rather than individual handler internals.

// Tests use `panic!` in match arms for explicit scenario failure reporting.
#![allow(clippy::panic)]

use perl_dap::feature_catalog::has_feature;
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};

fn new_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

fn expect_response(
    msg: DapMessage,
    expected_command: &str,
) -> (bool, Option<Value>, Option<String>) {
    match msg {
        DapMessage::Response { success, command, body, message, .. } => {
            assert_eq!(command, expected_command, "response command echo mismatch");
            (success, body, message)
        }
        other => panic!("expected response for {expected_command}, got {other:?}"),
    }
}

#[test]
fn scenario_initialize_negotiates_capabilities_from_feature_catalog() {
    // Given: a fresh DAP adapter
    let mut adapter = new_adapter();

    // When: the client sends initialize
    let (success, body, _message) =
        expect_response(adapter.handle_request(1, "initialize", None), "initialize");

    // Then: initialization succeeds and advertised capabilities mirror feature flags
    assert!(success, "initialize should succeed");
    let body = body.expect("initialize must return a body");
    let capabilities =
        body.as_object().expect("initialize response body must be a capabilities object");

    let supports_modules = capabilities
        .get("supportsModulesRequest")
        .and_then(|v| v.as_bool())
        .expect("supportsModulesRequest must be present");
    assert_eq!(supports_modules, has_feature("dap.modules"));

    let supports_data_breakpoints = capabilities
        .get("supportsDataBreakpoints")
        .and_then(|v| v.as_bool())
        .expect("supportsDataBreakpoints must be present");
    assert_eq!(supports_data_breakpoints, has_feature("dap.watchpoints"));

    let supports_completions = capabilities
        .get("supportsCompletionsRequest")
        .and_then(|v| v.as_bool())
        .expect("supportsCompletionsRequest must be present");
    assert_eq!(supports_completions, has_feature("dap.completions"));
}

#[test]
fn scenario_pre_launch_introspection_requests_are_safe_and_well_formed() {
    // Given: a fresh adapter with no active debuggee process
    let mut adapter = new_adapter();

    // When: the client asks for threads and stack trace before launch/attach
    let (threads_success, threads_body, _) =
        expect_response(adapter.handle_request(2, "threads", None), "threads");
    let (stack_success, stack_body, _) = expect_response(
        adapter.handle_request(3, "stackTrace", Some(json!({"threadId": 1}))),
        "stackTrace",
    );

    // Then: both requests succeed with well-formed collections instead of crashing/failing
    assert!(threads_success, "threads should succeed without active session");
    let threads_body = threads_body.expect("threads must return a body");
    let threads = threads_body
        .get("threads")
        .and_then(|v| v.as_array())
        .expect("threads body must contain threads array");
    assert!(threads.is_empty(), "threads should be empty before launch/attach");

    assert!(stack_success, "stackTrace should succeed without active session");
    let stack_body = stack_body.expect("stackTrace must return a body");
    let stack_frames = stack_body
        .get("stackFrames")
        .and_then(|v| v.as_array())
        .expect("stackTrace body must contain stackFrames array");
    assert!(
        stack_frames.len() <= 1,
        "stackTrace should not fabricate deep stacks before launch/attach"
    );
}

#[test]
fn scenario_unknown_command_returns_structured_protocol_error() {
    // Given: a fresh adapter
    let mut adapter = new_adapter();

    // When: the client sends an unknown method
    let (success, _body, message) = expect_response(
        adapter.handle_request(99, "totallyUnknownMethod", None),
        "totallyUnknownMethod",
    );

    // Then: request fails in a controlled way with a useful error message
    assert!(!success, "unknown commands must fail");
    let msg = message.unwrap_or_default().to_lowercase();
    assert!(
        msg.contains("unknown") || msg.contains("not implemented"),
        "error should explain unknown command"
    );
}
