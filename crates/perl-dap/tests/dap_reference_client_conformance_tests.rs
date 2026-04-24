//! Reference-client conformance sweep for DAP.
//!
//! Replays representative reference-client request streams and verifies the
//! adapter returns spec-shaped responses with correctly ordered side-band events.

use anyhow::{Result, anyhow};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reference_clients").join(name)
}

fn load_fixture(name: &str) -> Result<Value> {
    let raw = std::fs::read_to_string(fixture_path(name))?;
    Ok(serde_json::from_str(&raw)?)
}

fn resolve_workspace_vars(value: &Value) -> Value {
    let workspace_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").to_string_lossy().to_string();
    match value {
        Value::String(s) => Value::String(s.replace("${workspaceFolder}", &workspace_root)),
        Value::Array(items) => Value::Array(items.iter().map(resolve_workspace_vars).collect()),
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), resolve_workspace_vars(v))).collect())
        }
        _ => value.clone(),
    }
}

fn drain_events(receiver: &Receiver<DapMessage>) -> Vec<DapMessage> {
    receiver.try_iter().collect()
}

fn assert_required_body_keys(
    command: &str,
    body: &Value,
    required_body_keys: &[Value],
) -> Result<()> {
    for key in required_body_keys {
        let key =
            key.as_str().ok_or_else(|| anyhow!("requiredBodyKeys entries must be strings"))?;
        if body.get(key).is_none() {
            return Err(anyhow!("{command} response body missing required key: {key}"));
        }
    }
    Ok(())
}

fn assert_expected_events(
    command: &str,
    expected_events: Option<&Vec<Value>>,
    emitted_events: &[DapMessage],
) -> Result<()> {
    let expected_events = expected_events.cloned().unwrap_or_default();
    if emitted_events.len() != expected_events.len() {
        return Err(anyhow!(
            "{command} emitted {} event(s) but fixture expected {}",
            emitted_events.len(),
            expected_events.len()
        ));
    }

    let mut prev_event_seq = 0_i64;
    for (event_idx, (expected_event, actual_event)) in
        expected_events.iter().zip(emitted_events.iter()).enumerate()
    {
        let expected_name = expected_event
            .get("event")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("expectedEvents[{event_idx}] missing event name"))?;

        match actual_event {
            DapMessage::Event { seq, event, body } => {
                if *seq <= prev_event_seq {
                    return Err(anyhow!(
                        "{command} event seq must increase monotonically: {seq} <= {prev_event_seq}"
                    ));
                }
                prev_event_seq = *seq;

                if event != expected_name {
                    return Err(anyhow!(
                        "{command} event ordering mismatch at index {event_idx}: expected {expected_name}, got {event}"
                    ));
                }

                if let Some(required_body_keys) =
                    expected_event.get("requiredBodyKeys").and_then(Value::as_array)
                {
                    let body = body
                        .as_ref()
                        .ok_or_else(|| anyhow!("{event} event missing required body"))?;
                    assert_required_body_keys(event, body, required_body_keys)?;
                }
            }
            other => {
                return Err(anyhow!(
                    "{command} expected DAP event at index {event_idx}, got {other:?}"
                ));
            }
        }
    }

    Ok(())
}

fn run_fixture(fixture_name: &str) -> Result<()> {
    let fixture = load_fixture(fixture_name)?;
    let requests = fixture
        .get("requests")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("fixture requests must be an array"))?;

    let mut adapter = DebugAdapter::new();
    let (sender, receiver) = channel();
    adapter.set_event_sender(sender);

    let mut prev_response_seq = 0_i64;

    for (idx, request) in requests.iter().enumerate() {
        let request_seq = (idx as i64) + 1;
        let command = request
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("request entry missing command"))?;
        let arguments = request.get("arguments").map(resolve_workspace_vars);
        let expected_success =
            request.get("expectSuccess").and_then(Value::as_bool).unwrap_or(true);

        let response = adapter.handle_request(request_seq, command, arguments);
        match response {
            DapMessage::Response {
                seq,
                request_seq: echoed_request_seq,
                success,
                command: echoed_command,
                body,
                message,
            } => {
                assert!(
                    seq > prev_response_seq,
                    "response seq must increase monotonically for {command}"
                );
                prev_response_seq = seq;
                assert_eq!(
                    echoed_request_seq, request_seq,
                    "request_seq echo mismatch for {command}"
                );
                assert_eq!(echoed_command, command, "command echo mismatch for {command}");
                assert_eq!(
                    success, expected_success,
                    "success mismatch for {command}: {message:?}"
                );

                if let Some(ref response_body) = body {
                    assert!(
                        response_body.is_object(),
                        "{command} response body must be an object when present"
                    );
                }

                if let Some(required_message_substring) =
                    request.get("requiredMessageContains").and_then(Value::as_str)
                {
                    let message = message.unwrap_or_default();
                    assert!(
                        message.contains(required_message_substring),
                        "{command} response message must contain '{required_message_substring}', got: {message}"
                    );
                }

                if let Some(required_body_keys) =
                    request.get("requiredBodyKeys").and_then(Value::as_array)
                {
                    let response_body =
                        body.ok_or_else(|| anyhow!("{command} response missing body"))?;
                    assert_required_body_keys(command, &response_body, required_body_keys)?;
                }
            }
            other => {
                return Err(anyhow!("expected response for {command}, got {other:?}"));
            }
        }

        let emitted_events = drain_events(&receiver);
        assert_expected_events(
            command,
            request.get("expectedEvents").and_then(Value::as_array),
            &emitted_events,
        )?;
    }

    Ok(())
}

#[test]
fn vscode_mock_debug_surface_conformance() -> Result<()> {
    run_fixture("vscode_mock_debug_smoke.json")
}

#[test]
fn vscode_attach_protocol_conformance() -> Result<()> {
    run_fixture("vscode_attach_conformance.json")
}
