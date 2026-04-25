//! Reference-client conformance sweep for DAP.
//!
//! Replays a VS Code mock-debug-style request stream and verifies the adapter
//! returns spec-shaped responses across the command surface.

use anyhow::{Result, anyhow};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reference_clients")
}

fn load_fixtures() -> Result<Vec<(String, Value)>> {
    let mut fixtures = Vec::new();
    for entry in std::fs::read_dir(fixture_dir())? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)?;
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| anyhow!("fixture path is not valid UTF-8: {path:?}"))?
            .to_string();
        fixtures.push((name, serde_json::from_str(&raw)?));
    }
    fixtures.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    Ok(fixtures)
}

fn collect_events(rx: &Receiver<DapMessage>, timeout_ms: u64) -> Vec<(String, Option<Value>)> {
    let mut events = Vec::new();
    if let Ok(message) = rx.recv_timeout(Duration::from_millis(timeout_ms)) {
        if let DapMessage::Event { event, body, .. } = message {
            events.push((event, body));
        }
    }
    while let Ok(message) = rx.try_recv() {
        if let DapMessage::Event { event, body, .. } = message {
            events.push((event, body));
        }
    }
    events
}

fn assert_expected_events(
    fixture_name: &str,
    command: &str,
    idx: usize,
    expected: &[Value],
    observed: &[(String, Option<Value>)],
) -> Result<()> {
    let observed_names: Vec<&str> = observed.iter().map(|(name, _)| name.as_str()).collect();
    let mut cursor = 0_usize;
    for event in expected {
        let name = event.get("event").and_then(Value::as_str).ok_or_else(|| {
            anyhow!("{fixture_name}[{idx}] expectedEventsAfter entry missing event")
        })?;
        let required_body_keys =
            event.get("requiredBodyKeys").and_then(Value::as_array).cloned().unwrap_or_default();
        let offset = observed_names[cursor..].iter().position(|candidate| *candidate == name);
        let found_at = match offset {
            Some(offset) => cursor + offset,
            None => {
                return Err(anyhow!(
                    "{fixture_name}[{idx}] {command}: missing expected event '{name}', observed: {observed_names:?}"
                ));
            }
        };
        let body = observed[found_at].1.as_ref();
        for key in required_body_keys {
            let key = key.as_str().ok_or_else(|| {
                anyhow!("{fixture_name}[{idx}] expected event requiredBodyKeys must be strings")
            })?;
            let body = body.ok_or_else(|| {
                anyhow!("{fixture_name}[{idx}] {command}: event '{name}' missing body")
            })?;
            let obj = body.as_object().ok_or_else(|| {
                anyhow!("{fixture_name}[{idx}] {command}: event '{name}' body must be an object")
            })?;
            if !obj.contains_key(key) {
                return Err(anyhow!(
                    "{fixture_name}[{idx}] {command}: event '{name}' body missing key '{key}'"
                ));
            }
        }
        cursor = found_at + 1;
    }
    Ok(())
}

#[test]
fn vscode_mock_debug_surface_conformance() -> Result<()> {
    let fixtures = load_fixtures()?;
    for (fixture_name, fixture) in fixtures {
        let requests = fixture
            .get("requests")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("{fixture_name}: fixture requests must be an array"))?;

        let (tx, rx) = channel();
        let mut adapter = DebugAdapter::new();
        adapter.set_event_sender(tx);
        let mut prev_response_seq = 0_i64;

        for (idx, request) in requests.iter().enumerate() {
            let request_seq =
                request.get("requestSeq").and_then(Value::as_i64).unwrap_or((idx as i64) + 1);
            let command = request
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("{fixture_name}[{idx}]: request entry missing command"))?;
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
                        "{fixture_name}[{idx}] {command}: response seq must increase monotonically"
                    );
                    prev_response_seq = seq;
                    assert_eq!(
                        echoed_request_seq, request_seq,
                        "{fixture_name}[{idx}] {command}: request_seq echo mismatch"
                    );
                    assert_eq!(
                        echoed_command, command,
                        "{fixture_name}[{idx}] {command}: command echo mismatch"
                    );
                    assert_eq!(
                        success, expected_success,
                        "{fixture_name}[{idx}] {command}: success mismatch: {message:?}"
                    );

                    if let Some(body) = &body {
                        assert!(
                            body.is_object(),
                            "{fixture_name}[{idx}] {command}: response body must be an object when present"
                        );
                    }

                    if let Some(required_message_substring) =
                        request.get("requiredMessageContains").and_then(Value::as_str)
                    {
                        let message = message.unwrap_or_default();
                        assert!(
                            message.contains(required_message_substring),
                            "{fixture_name}[{idx}] {command}: response message must contain '{required_message_substring}', got: {message}"
                        );
                    }

                    if let Some(required_body_keys) =
                        request.get("requiredBodyKeys").and_then(Value::as_array)
                    {
                        let body = body.ok_or_else(|| {
                            anyhow!("{fixture_name}[{idx}] {command} response missing body")
                        })?;
                        let obj = body.as_object().ok_or_else(|| {
                            anyhow!("{fixture_name}[{idx}] {command} response body must be object")
                        })?;
                        for key in required_body_keys {
                            let key = key.as_str().ok_or_else(|| {
                                anyhow!(
                                    "{fixture_name}[{idx}] requiredBodyKeys entries must be strings"
                                )
                            })?;
                            assert!(
                                obj.contains_key(key),
                                "{fixture_name}[{idx}] {command}: response body missing required key: {key}"
                            );
                        }
                    }
                }
                other => {
                    return Err(anyhow!(
                        "{fixture_name}[{idx}] expected response for {command}, got {other:?}"
                    ));
                }
            }

            if let Some(expected_events) =
                request.get("expectedEventsAfter").and_then(Value::as_array)
            {
                let observed_events = collect_events(&rx, 100);
                assert_expected_events(
                    &fixture_name,
                    command,
                    idx,
                    expected_events,
                    &observed_events,
                )?;
            }
        }
    }
    Ok(())
}
