//! DAP Golden Transcript Tests (AC13)
//!
//! Validates transcript fixtures and replays representative command flows.
//!
//! Run with: `cargo test -p perl-dap --features dap-phase2`

#[cfg(feature = "dap-phase2")]
mod dap_golden_transcripts {
    use anyhow::Result;
    use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
    use serde_json::{Value, json};
    use std::path::PathBuf;

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
}
