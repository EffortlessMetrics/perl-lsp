//! Reference-client conformance sweep for DAP.
//!
//! Replays a VS Code mock-debug-style request stream and verifies the adapter
//! returns spec-shaped responses across the command surface.

use anyhow::{Result, anyhow};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::Value;
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/reference_clients/vscode_mock_debug_smoke.json")
}

fn load_fixture() -> Result<Value> {
    let raw = std::fs::read_to_string(fixture_path())?;
    Ok(serde_json::from_str(&raw)?)
}

#[test]
fn vscode_mock_debug_surface_conformance() -> Result<()> {
    let fixture = load_fixture()?;
    let requests = fixture
        .get("requests")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("fixture requests must be an array"))?;

    let mut adapter = DebugAdapter::new();
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
                    let body = body.ok_or_else(|| anyhow!("{command} response missing body"))?;
                    for key in required_body_keys {
                        let key = key
                            .as_str()
                            .ok_or_else(|| anyhow!("requiredBodyKeys entries must be strings"))?;
                        assert!(
                            body.get(key).is_some(),
                            "{command} response body missing required key: {key}"
                        );
                    }
                }
            }
            other => {
                return Err(anyhow!("expected response for {command}, got {other:?}"));
            }
        }
    }

    Ok(())
}
