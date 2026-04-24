//! DAP Golden Transcript Tests (AC13)
//!
//! Validates transcript fixtures and replays representative command flows.
//!
//! Run with: `cargo test -p perl-dap --features dap-phase2 -- golden`

#[cfg(feature = "dap-phase2")]
mod dap_golden_transcripts {
    use anyhow::{Result, anyhow};
    use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
    use serde_json::{Value, json};
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, channel};

    fn transcript_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/golden_transcripts/{name}"))
    }

    fn load_transcript(name: &str) -> Result<Value> {
        let path = transcript_path(name);
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    fn extract_messages(transcript: &Value) -> Result<&Vec<Value>> {
        transcript
            .get("messages")
            .or_else(|| transcript.get("sequence"))
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("transcript missing messages/sequence array"))
    }

    fn resolve_workspace_vars(value: &Value) -> Value {
        let workspace_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").to_string_lossy().to_string();
        match value {
            Value::String(s) => Value::String(s.replace("${workspaceFolder}", &workspace_root)),
            Value::Array(items) => Value::Array(items.iter().map(resolve_workspace_vars).collect()),
            Value::Object(map) => Value::Object(
                map.iter().map(|(k, v)| (k.clone(), resolve_workspace_vars(v))).collect(),
            ),
            _ => value.clone(),
        }
    }

    fn send_and_expect_success(
        adapter: &mut DebugAdapter,
        request_seq: i64,
        command: &str,
        arguments: Option<Value>,
    ) -> Result<()> {
        let response = adapter.handle_request(request_seq, command, arguments);
        match response {
            DapMessage::Response { success, command: actual, .. } => {
                if !success {
                    anyhow::bail!("expected success for {command}, got failure");
                }
                if actual != command {
                    anyhow::bail!("expected {command} response, got {actual}");
                }
            }
            _ => anyhow::bail!("expected response for {command}"),
        }
        Ok(())
    }

    fn drain_events(receiver: &Receiver<DapMessage>) -> Vec<DapMessage> {
        let mut events = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            events.push(message);
        }
        events
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-hello-world-transcript
    #[tokio::test]
    // AC:13
    async fn test_hello_world_golden_transcript() -> Result<()> {
        let transcript = load_transcript("hello_expected.json")?;
        let sequence = extract_messages(&transcript)?;
        assert!(sequence.iter().any(|m| m["command"] == "initialize"));
        assert!(sequence.iter().any(|m| m["command"] == "setBreakpoints"));
        assert!(sequence.iter().any(|m| m["command"] == "disconnect"));

        let mut adapter = DebugAdapter::new();
        send_and_expect_success(&mut adapter, 1, "initialize", None)?;

        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello.pl");
        send_and_expect_success(
            &mut adapter,
            2,
            "setBreakpoints",
            Some(json!({
                "source": { "path": fixture },
                "breakpoints": [{ "line": 9 }]
            })),
        )?;
        send_and_expect_success(&mut adapter, 3, "continue", Some(json!({ "threadId": 1 })))?;
        send_and_expect_success(&mut adapter, 4, "stackTrace", Some(json!({ "threadId": 1 })))?;
        send_and_expect_success(&mut adapter, 5, "disconnect", None)?;
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-step-through-transcript
    #[tokio::test]
    // AC:13
    async fn test_step_through_golden_transcript() -> Result<()> {
        let transcript = load_transcript("stepping_sequence.json")?;
        let messages = extract_messages(&transcript)?;
        assert!(messages.iter().any(|m| m["command"] == "continue"));
        assert!(messages.iter().any(|m| m["command"] == "next"));
        assert!(messages.iter().any(|m| m["command"] == "stepIn"));
        assert!(messages.iter().any(|m| m["command"] == "stepOut"));

        let mut adapter = DebugAdapter::new();
        send_and_expect_success(&mut adapter, 1, "continue", Some(json!({ "threadId": 1 })))?;
        send_and_expect_success(&mut adapter, 2, "next", Some(json!({ "threadId": 1 })))?;
        send_and_expect_success(&mut adapter, 3, "stepIn", Some(json!({ "threadId": 1 })))?;
        send_and_expect_success(&mut adapter, 4, "stepOut", Some(json!({ "threadId": 1 })))?;
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-module-debugging-transcript
    #[tokio::test]
    // AC:13
    async fn test_module_debugging_golden_transcript() -> Result<()> {
        let transcript = load_transcript("launch_attach_sequence.json")?;
        let messages = extract_messages(&transcript)?;
        assert!(messages.iter().any(|m| m["command"] == "launch"));
        assert!(messages.iter().any(|m| m["event"] == "stopped"));

        // Validate placeholder substitution can be resolved for execution contexts.
        let launch_request = messages
            .iter()
            .find(|m| m["type"] == "request" && m["command"] == "launch")
            .ok_or_else(|| anyhow::anyhow!("launch request missing from transcript"))?;
        let resolved = resolve_workspace_vars(launch_request);
        assert!(resolved["arguments"]["program"].is_string());

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-evaluate-transcript
    #[tokio::test]
    // AC:13
    async fn test_evaluate_expressions_golden_transcript() -> Result<()> {
        let transcript = load_transcript("variable_sequence.json")?;
        let messages = extract_messages(&transcript)?;
        assert!(messages.iter().any(|m| m["command"] == "stackTrace"));
        assert!(messages.iter().any(|m| m["command"] == "scopes"));
        assert!(messages.iter().any(|m| m["command"] == "variables"));

        let mut adapter = DebugAdapter::new();
        send_and_expect_success(&mut adapter, 1, "stackTrace", Some(json!({ "threadId": 1 })))?;
        send_and_expect_success(&mut adapter, 2, "scopes", Some(json!({ "frameId": 1 })))?;
        send_and_expect_success(
            &mut adapter,
            3,
            "variables",
            Some(json!({ "variablesReference": 11 })),
        )?;

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac13-error-handling-transcript
    #[tokio::test]
    // AC:13
    async fn test_error_handling_golden_transcript() -> Result<()> {
        let transcript = load_transcript("breakpoint_sequence.json")?;
        let messages = extract_messages(&transcript)?;
        assert!(messages.iter().any(|m| m["command"] == "setBreakpoints"));

        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(
            1,
            "setBreakpoints",
            Some(json!({
                "source": { "path": "/nonexistent/script.pl" },
                "breakpoints": [{ "line": 999 }]
            })),
        );
        match response {
            DapMessage::Response { success, body, .. } => {
                assert!(success, "request should succeed with unverified breakpoint payload");
                let body = body.ok_or_else(|| anyhow::anyhow!("missing setBreakpoints body"))?;
                let bps = body["breakpoints"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("missing breakpoints array"))?;
                assert_eq!(bps.len(), 1);
                assert!(
                    !bps[0]["verified"].as_bool().unwrap_or(true),
                    "nonexistent file should produce unverified breakpoint"
                );
            }
            _ => anyhow::bail!("expected setBreakpoints response"),
        }
        Ok(())
    }

    /// Tests richer event/response conformance with an attach-driven session flow.
    #[tokio::test]
    // AC:13
    async fn test_attach_rich_session_golden_transcript() -> Result<()> {
        let transcript = load_transcript("attach_rich_session_sequence.json")?;
        let messages = extract_messages(&transcript)?;

        let required_commands = [
            "initialize",
            "attach",
            "setBreakpoints",
            "configurationDone",
            "stackTrace",
            "scopes",
            "variables",
            "evaluate",
            "continue",
            "disconnect",
        ];

        for command in required_commands {
            assert!(
                messages.iter().any(|m| m["type"] == "request" && m["command"] == command),
                "transcript should contain request command '{command}'"
            );
        }

        let mut adapter = DebugAdapter::new();
        let (event_sender, event_receiver) = channel::<DapMessage>();
        adapter.set_event_sender(event_sender);

        let mut request_seq = 1_i64;
        let mut prev_response_seq = 0_i64;
        let mut event_log = Vec::new();

        for message in messages {
            if message.get("type").and_then(Value::as_str) != Some("request") {
                continue;
            }

            let command = message
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("request entry missing command"))?;
            let arguments = message.get("arguments").cloned().map(|v| resolve_workspace_vars(&v));

            let response = adapter.handle_request(request_seq, command, arguments);
            match response {
                DapMessage::Response {
                    seq,
                    request_seq: echoed_request_seq,
                    success,
                    command: echoed_command,
                    body,
                    ..
                } => {
                    assert!(seq > prev_response_seq, "response seq must increase for {command}");
                    prev_response_seq = seq;
                    assert_eq!(
                        echoed_request_seq, request_seq,
                        "request_seq mismatch for {command}"
                    );
                    assert_eq!(echoed_command, command, "command echo mismatch for {command}");

                    let serialized = serde_json::to_value(&DapMessage::Response {
                        seq,
                        request_seq: echoed_request_seq,
                        success,
                        command: echoed_command.clone(),
                        body: body.clone(),
                        message: None,
                    })?;
                    let object = serialized
                        .as_object()
                        .ok_or_else(|| anyhow!("{command} response should serialize as object"))?;
                    for key in ["type", "seq", "request_seq", "success", "command"] {
                        assert!(object.contains_key(key), "{command} response missing key '{key}'");
                    }

                    if let Some(expected_response) = messages.iter().find(|candidate| {
                        candidate["type"] == "response"
                            && candidate["command"] == command
                            && candidate["request_seq"] == request_seq
                    }) {
                        let expected_success =
                            expected_response["success"].as_bool().unwrap_or(success);
                        assert_eq!(
                            success, expected_success,
                            "success mismatch for {command}; expected transcript {expected_success}, got {success}"
                        );

                        if let Some(required_body_keys) =
                            expected_response.get("requiredBodyKeys").and_then(Value::as_array)
                        {
                            let response_body = body
                                .as_ref()
                                .ok_or_else(|| anyhow!("{command} expected response body"))?;
                            for key in required_body_keys {
                                let key = key.as_str().ok_or_else(|| {
                                    anyhow!("requiredBodyKeys entry must be a string")
                                })?;
                                assert!(
                                    response_body.get(key).is_some(),
                                    "{command} response body missing key '{key}'"
                                );
                            }
                        }
                    }
                }
                other => anyhow::bail!("expected response for {command}, got {other:?}"),
            }

            for event in drain_events(&event_receiver) {
                if let DapMessage::Event { event, .. } = event {
                    event_log.push(event);
                }
            }
            request_seq += 1;
        }

        let expected_event_order =
            transcript["expectedEventOrder"].as_array().ok_or_else(|| {
                anyhow!("attach_rich_session_sequence transcript missing expectedEventOrder")
            })?;
        let expected_event_order = expected_event_order
            .iter()
            .map(|event| {
                event.as_str().ok_or_else(|| anyhow!("expectedEventOrder entries must be strings"))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut cursor = 0_usize;
        for expected_event in expected_event_order {
            let Some(position) =
                event_log[cursor..].iter().position(|event| event == expected_event)
            else {
                anyhow::bail!(
                    "expected event '{expected_event}' not found in event log: {event_log:?}"
                );
            };
            cursor += position + 1;
        }

        Ok(())
    }
}
