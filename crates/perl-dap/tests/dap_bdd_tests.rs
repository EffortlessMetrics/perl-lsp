//! BDD-style acceptance tests for core perl-dap request flows.
//!
//! These tests intentionally follow a Given/When/Then narrative so new
//! contributors can quickly map test intent to observable adapter behavior.

#![allow(clippy::panic)]

use perl_dap::{DapMessage, DebugAdapter, feature_catalog};
use serde_json::{Value, json};

fn when_request(
    adapter: &mut DebugAdapter,
    request_seq: i64,
    command: &str,
    arguments: Option<Value>,
) -> DapMessage {
    adapter.handle_request(request_seq, command, arguments)
}

fn then_response_success(msg: DapMessage, expected_command: &str) -> Option<Value> {
    match msg {
        DapMessage::Response { success, command, body, message, .. } => {
            assert_eq!(command, expected_command, "response command should match request");
            assert!(success, "expected `{expected_command}` to succeed, got message: {message:?}");
            body
        }
        other => panic!("expected Response message, got {other:?}"),
    }
}

fn then_response_failure(msg: DapMessage, expected_command: &str) -> String {
    match msg {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, expected_command, "response command should match request");
            assert!(!success, "expected `{expected_command}` to fail");
            message.unwrap_or_default()
        }
        other => panic!("expected Response message, got {other:?}"),
    }
}

#[test]
fn scenario_initialize_advertises_catalog_backed_capabilities() {
    // Given a fresh adapter with no active debug session.
    let mut adapter = DebugAdapter::new();

    // When the client sends initialize.
    let initialize = when_request(
        &mut adapter,
        1,
        "initialize",
        Some(json!({
            "clientID": "bdd-test",
            "adapterID": "perl-dap"
        })),
    );

    // Then the capability advertisement mirrors the feature catalog.
    let body = then_response_success(initialize, "initialize")
        .expect("initialize must include response body");

    let supports_configuration_done =
        body.get("supportsConfigurationDoneRequest").and_then(Value::as_bool).unwrap_or(false);
    assert_eq!(
        supports_configuration_done,
        feature_catalog::has_feature("dap.core"),
        "supportsConfigurationDoneRequest must mirror dap.core feature flag"
    );

    let supports_log_points =
        body.get("supportsLogPoints").and_then(Value::as_bool).unwrap_or(false);
    assert_eq!(
        supports_log_points,
        feature_catalog::has_feature("dap.breakpoints.logpoints"),
        "supportsLogPoints must mirror dap.breakpoints.logpoints feature flag"
    );
}

#[test]
fn scenario_launch_without_program_is_rejected() {
    // Given a fresh adapter.
    let mut adapter = DebugAdapter::new();

    // When launch is requested without a program path.
    let launch = when_request(&mut adapter, 2, "launch", Some(json!({ "stopOnEntry": false })));

    // Then the adapter returns a structured failure explaining invalid launch args.
    let message = then_response_failure(launch, "launch");
    assert!(
        message.to_lowercase().contains("program") || message.to_lowercase().contains("missing"),
        "launch error should explain missing program argument: {message}"
    );
}

#[test]
fn scenario_cancel_is_idempotent_before_session_start() {
    // Given a fresh adapter with no active debuggee process.
    let mut adapter = DebugAdapter::new();

    // When cancel is sent multiple times.
    let first_cancel = when_request(&mut adapter, 3, "cancel", None);
    let second_cancel = when_request(&mut adapter, 4, "cancel", Some(json!({ "requestId": 3 })));

    // Then each cancel request succeeds.
    let first_body = then_response_success(first_cancel, "cancel");
    let second_body = then_response_success(second_cancel, "cancel");
    assert!(first_body.is_none(), "cancel should not return a response body");
    assert!(second_body.is_none(), "cancel should not return a response body");
}

#[test]
fn scenario_exception_info_without_session_returns_safe_fallback() {
    // Given a fresh adapter with no exception context.
    let mut adapter = DebugAdapter::new();

    // When exceptionInfo is requested.
    let exception_info =
        when_request(&mut adapter, 5, "exceptionInfo", Some(json!({ "threadId": 1 })));

    // Then the adapter responds successfully with required fallback fields.
    let body = then_response_success(exception_info, "exceptionInfo")
        .expect("exceptionInfo must include response body");

    assert!(body.get("exceptionId").is_some(), "exceptionInfo must include exceptionId");
    assert!(body.get("breakMode").is_some(), "exceptionInfo must include breakMode");
}
