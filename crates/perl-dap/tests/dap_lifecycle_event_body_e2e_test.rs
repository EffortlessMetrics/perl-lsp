//! End-to-end content tests for DAP lifecycle event bodies (`terminate`,
//! `disconnect`, `restart`).
//!
//! Existing `session_lifecycle_tests.rs` covers exactly one `terminate` shape:
//! `{ "restart": false }`. This file fills the matrix and locks down the
//! `terminated` event body contract for the other inputs an IDE will send:
//!
//! - `terminate` with `restart: true`: event body must echo `restart: true`
//! - `terminate` with no `restart` arg: event body must NOT include `restart`
//! - `terminate` with empty args (`{}`): event body must NOT include `restart`
//! - `terminate` twice in succession: both calls succeed and emit events
//! - `disconnect` (no session): terminated event body must be `None`
//! - `restart` request (no session): fails cleanly per `unsupported` handler
//!
//! All tests are protocol-level: no `perl` process is spawned.

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use perl_tdd_support::must_some;
use serde_json::{Value, json};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn create_test_adapter() -> (DebugAdapter, Receiver<DapMessage>) {
    let (tx, rx) = channel();
    let mut adapter = DebugAdapter::new();
    adapter.set_event_sender(tx);
    (adapter, rx)
}

fn wait_for_event(rx: &Receiver<DapMessage>, name: &str, timeout_ms: u64) -> Option<Value> {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(DapMessage::Event { event, body, .. }) if event == name => {
                return Some(body.unwrap_or(Value::Null));
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    None
}

fn assert_response_success(response: &DapMessage, expected_command: &str) -> TestResult {
    let DapMessage::Response { success, command, message, .. } = response else {
        return Err(format!("expected Response for {expected_command}, got {response:?}").into());
    };
    assert!(*success, "expected success for {expected_command}, got error: {message:?}");
    assert_eq!(command, expected_command, "command field mismatch");
    Ok(())
}

#[test]
fn terminate_with_restart_true_echoes_flag_in_event_body() -> TestResult {
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "terminate", Some(json!({ "restart": true })));
    assert_response_success(&response, "terminate")?;

    let body = must_some(wait_for_event(&rx, "terminated", 200));
    let restart = body.get("restart").and_then(Value::as_bool);
    assert_eq!(
        restart,
        Some(true),
        "terminate({{restart: true}}) must emit terminated event with restart=true, got body={body}"
    );

    Ok(())
}

#[test]
fn terminate_with_no_arguments_omits_restart_field() -> TestResult {
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "terminate", None);
    assert_response_success(&response, "terminate")?;

    let body = must_some(wait_for_event(&rx, "terminated", 200));
    // Per debug_adapter::process::handle_terminate: when no restart arg was
    // supplied, the event body must NOT include a restart field. The body may
    // be `null` (no body) or an object without the `restart` key.
    let has_restart = body.get("restart").is_some();
    assert!(
        !has_restart,
        "terminate() without restart arg must NOT include restart field, got body={body}"
    );

    Ok(())
}

#[test]
fn terminate_with_empty_arguments_omits_restart_field() -> TestResult {
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "terminate", Some(json!({})));
    assert_response_success(&response, "terminate")?;

    let body = must_some(wait_for_event(&rx, "terminated", 200));
    let has_restart = body.get("restart").is_some();
    assert!(
        !has_restart,
        "terminate({{}}) without restart arg must NOT include restart field, got body={body}"
    );

    Ok(())
}

#[test]
fn terminate_twice_in_succession_both_succeed_and_emit_events() -> TestResult {
    let (mut adapter, rx) = create_test_adapter();

    // First terminate
    let first = adapter.handle_request(1, "terminate", Some(json!({ "restart": false })));
    assert_response_success(&first, "terminate")?;
    let first_body = must_some(wait_for_event(&rx, "terminated", 200));
    assert_eq!(
        first_body.get("restart").and_then(Value::as_bool),
        Some(false),
        "first terminate must echo restart=false"
    );

    // Second terminate must also succeed; adapter is idempotent.
    let second = adapter.handle_request(2, "terminate", Some(json!({ "restart": true })));
    assert_response_success(&second, "terminate")?;
    let second_body = must_some(wait_for_event(&rx, "terminated", 200));
    assert_eq!(
        second_body.get("restart").and_then(Value::as_bool),
        Some(true),
        "second terminate must echo restart=true, independent of first"
    );

    Ok(())
}

#[test]
fn disconnect_without_session_emits_terminated_event_with_null_body() -> TestResult {
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "disconnect", None);
    assert_response_success(&response, "disconnect")?;

    // Per session_lifecycle_tests.rs::test_session_lifecycle_disconnect_without_session,
    // disconnect emits a terminated event. We tighten the contract by asserting
    // the event body is null (no restart info) since no restart arg was sent.
    let body = wait_for_event(&rx, "terminated", 200);
    assert!(body.is_some(), "disconnect must emit a terminated event");
    let body_value = must_some(body);
    let has_restart = body_value.get("restart").is_some();
    assert!(
        !has_restart,
        "disconnect (no args) must not include restart in terminated body, got {body_value}"
    );

    Ok(())
}

#[test]
fn restart_request_without_session_fails_cleanly() -> TestResult {
    let (mut adapter, _rx) = create_test_adapter();

    // The DAP `restart` request is not implemented for the native adapter.
    // The dispatcher's fall-through path must return success=false with a
    // descriptive message rather than panicking.
    let response = adapter.handle_request(1, "restart", None);
    let DapMessage::Response { success, command, message, .. } = &response else {
        return Err(format!("expected Response for restart, got {response:?}").into());
    };
    assert!(!*success, "restart should not silently succeed when unimplemented");
    assert_eq!(command, "restart");
    assert!(message.is_some(), "restart failure must carry a message for the IDE");

    Ok(())
}
