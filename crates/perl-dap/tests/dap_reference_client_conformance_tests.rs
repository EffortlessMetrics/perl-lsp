//! Reference-client conformance sweep for DAP.
//!
//! Replays VS Code mock-debug-style request streams and verifies the adapter
//! returns spec-shaped responses across the command surface.

use anyhow::{Result, anyhow};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reference_clients")
}

fn fixture_paths() -> Result<Vec<PathBuf>> {
    let mut paths = std::fs::read_dir(fixture_dir())?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn load_fixture(path: &PathBuf) -> Result<Value> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn collect_events(rx: &Receiver<DapMessage>) -> Vec<DapMessage> {
    let mut events = Vec::new();
    while let Ok(message) = rx.try_recv() {
        events.push(message);
    }
    events
}

fn assert_expected_events(
    command: &str,
    request: &Value,
    received_events: &[DapMessage],
) -> Result<()> {
    let expected_events = request
        .get("expectedEventsAfterRequest")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if expected_events.is_empty() {
        return Ok(());
    }

    let actual_events = received_events
        .iter()
        .map(|msg| match msg {
            DapMessage::Event { event, body, .. } => Some((event.as_str(), body)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| anyhow!("{command} emitted non-event messages in event stream"))?;

    assert_eq!(
        actual_events.len(),
        expected_events.len(),
        "{command} expected {} event(s) but got {}",
        expected_events.len(),
        actual_events.len()
    );

    for (idx, expected) in expected_events.iter().enumerate() {
        let expected_name = expected
            .get("event")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("expectedEventsAfterRequest[{idx}] missing event name"))?;
        let (actual_name, actual_body) = actual_events[idx];
        assert_eq!(actual_name, expected_name, "{command} event order mismatch at index {idx}");

        if let Some(required_body_keys) = expected.get("requiredBodyKeys").and_then(Value::as_array)
        {
            let body = actual_body
                .as_ref()
                .ok_or_else(|| anyhow!("{command} event '{actual_name}' missing body"))?;
            for key in required_body_keys {
                let key = key
                    .as_str()
                    .ok_or_else(|| anyhow!("event requiredBodyKeys entries must be strings"))?;
                assert!(
                    body.get(key).is_some(),
                    "{command} event '{actual_name}' body missing required key: {key}"
                );
            }
        }
    }

    Ok(())
}

fn assert_response_shape(command: &str, body: &Option<Value>, message: &Option<String>) {
    if let Some(payload) = body {
        assert!(payload.is_object(), "{command} response body must be a JSON object when present");
    }

    if let Some(text) = message {
        assert!(
            !text.trim().is_empty(),
            "{command} response message must not be empty when present"
        );
    }
}

#[test]
fn vscode_mock_debug_surface_conformance() -> Result<()> {
    for fixture_path in fixture_paths()? {
        let fixture = load_fixture(&fixture_path)?;
        let requests = fixture
            .get("requests")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("fixture requests must be an array"))?;

        let mut adapter = DebugAdapter::new();
        let (tx, rx) = channel();
        adapter.set_event_sender(tx);
        let mut prev_response_seq = 0_i64;

        for (idx, request) in requests.iter().enumerate() {
            let request_seq = (idx as i64) + 1;
            let command = request
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("request entry missing command"))?;
            let arguments = request.get("arguments").cloned();
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
                    assert_response_shape(command, &body, &message);

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
                        let body =
                            body.ok_or_else(|| anyhow!("{command} response missing body"))?;
                        for key in required_body_keys {
                            let key = key.as_str().ok_or_else(|| {
                                anyhow!("requiredBodyKeys entries must be strings")
                            })?;
                            assert!(
                                body.get(key).is_some(),
                                "{command} response body missing required key: {key}"
                            );
                        }
                    }

                    let emitted_events = collect_events(&rx);
                    assert_expected_events(command, request, &emitted_events)?;
                }
                other => {
                    return Err(anyhow!("expected response for {command}, got {other:?}"));
                }
            }
        }
    }

    Ok(())
}
