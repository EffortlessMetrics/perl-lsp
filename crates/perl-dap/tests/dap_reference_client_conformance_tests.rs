//! Reference-client conformance sweep for DAP.
//!
//! Replays reference-client style request streams and verifies the adapter
//! returns spec-shaped responses across the command surface.

use anyhow::{Result, anyhow};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reference_clients")
}

fn fixture_paths() -> Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(fixtures_dir())?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

fn load_fixture(path: &Path) -> Result<Value> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn drain_events(receiver: &Receiver<DapMessage>) -> Vec<DapMessage> {
    let mut events = Vec::new();
    while let Ok(message) = receiver.try_recv() {
        events.push(message);
    }
    events
}

fn assert_response_shape(response: &DapMessage, command: &str) -> Result<()> {
    let serialized = serde_json::to_value(response)?;
    let object = serialized
        .as_object()
        .ok_or_else(|| anyhow!("{command} response must serialize as an object"))?;

    for key in ["type", "seq", "request_seq", "success", "command"] {
        if !object.contains_key(key) {
            return Err(anyhow!("{command} response missing required top-level key: {key}"));
        }
    }

    if object.get("type").and_then(Value::as_str) != Some("response") {
        return Err(anyhow!("{command} response must have type='response'"));
    }

    Ok(())
}

fn assert_expected_events(
    request: &Value,
    emitted_events: &[DapMessage],
    command: &str,
    event_log: &mut Vec<String>,
) -> Result<()> {
    let Some(expected_events) = request.get("expectedEvents").and_then(Value::as_array) else {
        let allowed_implicit_event = match command {
            "initialize" => "initialized",
            "disconnect" => "terminated",
            _ => "",
        };
        let command_allows_implicit_event = !allowed_implicit_event.is_empty()
            && emitted_events.len() == 1
            && matches!(
                emitted_events.first(),
                Some(DapMessage::Event { event, .. }) if event == allowed_implicit_event
            );

        if !emitted_events.is_empty() && !command_allows_implicit_event {
            return Err(anyhow!("{command} emitted unexpected events: {emitted_events:?}"));
        }
        return Ok(());
    };

    if emitted_events.len() != expected_events.len() {
        return Err(anyhow!(
            "{command} emitted {} event(s), expected {}. Actual events: {emitted_events:?}",
            emitted_events.len(),
            expected_events.len(),
        ));
    }

    for (idx, (emitted, expected)) in emitted_events.iter().zip(expected_events.iter()).enumerate()
    {
        let expected_name = expected
            .get("event")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("expectedEvents[{idx}] missing event name for {command}"))?;

        match emitted {
            DapMessage::Event { event, body, .. } => {
                if event != expected_name {
                    return Err(anyhow!(
                        "{command} event order mismatch at index {idx}: expected '{expected_name}', got '{event}'"
                    ));
                }

                event_log.push(event.clone());

                if let Some(required_body_keys) =
                    expected.get("requiredBodyKeys").and_then(Value::as_array)
                {
                    let body = body
                        .as_ref()
                        .ok_or_else(|| anyhow!("{command} event '{event}' missing body"))?;
                    for key in required_body_keys {
                        let key = key.as_str().ok_or_else(|| {
                            anyhow!("{command} expectedEvents requiredBodyKeys must be strings")
                        })?;
                        if body.get(key).is_none() {
                            return Err(anyhow!(
                                "{command} event '{event}' body missing required key: {key}"
                            ));
                        }
                    }
                }

                if let Some(required_equals) =
                    expected.get("requiredBodyFieldEquals").and_then(Value::as_object)
                {
                    let body = body
                        .as_ref()
                        .ok_or_else(|| anyhow!("{command} event '{event}' missing body"))?;
                    for (key, expected_value) in required_equals {
                        let actual = body.get(key).ok_or_else(|| {
                            anyhow!("{command} event '{event}' body missing field: {key}")
                        })?;
                        if actual != expected_value {
                            return Err(anyhow!(
                                "{command} event '{event}' field '{key}' mismatch: expected {expected_value}, got {actual}"
                            ));
                        }
                    }
                }
            }
            other => {
                return Err(anyhow!(
                    "{command} expected event at index {idx}, got non-event message: {other:?}"
                ));
            }
        }
    }

    Ok(())
}

#[test]
fn vscode_reference_clients_surface_conformance() -> Result<()> {
    for fixture_path in fixture_paths()? {
        let fixture = load_fixture(&fixture_path)?;
        let fixture_name = fixture_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("invalid fixture name: {fixture_path:?}"))?;

        let requests = fixture
            .get("requests")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("{fixture_name}: fixture requests must be an array"))?;

        let mut adapter = DebugAdapter::new();
        let (event_sender, event_receiver) = channel::<DapMessage>();
        adapter.set_event_sender(event_sender);

        let mut prev_response_seq = 0_i64;
        let mut event_log = Vec::new();

        for (idx, request) in requests.iter().enumerate() {
            let request_seq = (idx as i64) + 1;
            let command = request
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("{fixture_name}: request entry missing command"))?;
            let arguments = request.get("arguments").cloned();
            let expected_success =
                request.get("expectSuccess").and_then(Value::as_bool).unwrap_or(true);

            let response = adapter.handle_request(request_seq, command, arguments);
            match &response {
                DapMessage::Response {
                    seq,
                    request_seq: echoed_request_seq,
                    success,
                    command: echoed_command,
                    body,
                    message,
                } => {
                    assert!(
                        *seq > prev_response_seq,
                        "{fixture_name}: response seq must increase monotonically for {command}"
                    );
                    prev_response_seq = *seq;
                    assert_eq!(
                        *echoed_request_seq, request_seq,
                        "{fixture_name}: request_seq echo mismatch for {command}"
                    );
                    assert_eq!(
                        echoed_command, command,
                        "{fixture_name}: command echo mismatch for {command}"
                    );
                    assert_eq!(
                        *success, expected_success,
                        "{fixture_name}: success mismatch for {command}: {message:?}"
                    );

                    assert_response_shape(&response, command)?;

                    if let Some(required_message_substring) =
                        request.get("requiredMessageContains").and_then(Value::as_str)
                    {
                        let message = message.clone().unwrap_or_default();
                        assert!(
                            message.contains(required_message_substring),
                            "{fixture_name}: {command} response message must contain '{required_message_substring}', got: {message}"
                        );
                    }

                    if let Some(required_body_keys) =
                        request.get("requiredBodyKeys").and_then(Value::as_array)
                    {
                        let body = body.as_ref().ok_or_else(|| {
                            anyhow!("{fixture_name}: {command} response missing body")
                        })?;
                        for key in required_body_keys {
                            let key = key.as_str().ok_or_else(|| {
                                anyhow!("{fixture_name}: requiredBodyKeys entries must be strings")
                            })?;
                            assert!(
                                body.get(key).is_some(),
                                "{fixture_name}: {command} response body missing required key: {key}"
                            );
                        }
                    }
                }
                other => {
                    return Err(anyhow!(
                        "{fixture_name}: expected response for {command}, got {other:?}"
                    ));
                }
            }

            let emitted_events = drain_events(&event_receiver);
            assert_expected_events(request, &emitted_events, command, &mut event_log)?;
        }

        if let Some(expected_event_order) =
            fixture.get("expectedEventOrder").and_then(Value::as_array)
        {
            let expected = expected_event_order
                .iter()
                .map(|value| {
                    value.as_str().ok_or_else(|| {
                        anyhow!("{fixture_name}: expectedEventOrder entries must be strings")
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let mut cursor = 0_usize;
            for expected_name in expected {
                if let Some(position) =
                    event_log[cursor..].iter().position(|event| event == expected_name)
                {
                    cursor += position + 1;
                } else {
                    return Err(anyhow!(
                        "{fixture_name}: expected event '{expected_name}' not found in emitted event log: {event_log:?}"
                    ));
                }
            }
        }
    }

    Ok(())
}
