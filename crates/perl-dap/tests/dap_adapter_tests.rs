//! DAP Native Adapter Tests (AC5-AC12)
//!
//! Tests for Phase 2 native adapter behavior.
//!
//! Run with: `cargo test -p perl-dap --features dap-phase2`

#[cfg(feature = "dap-phase2")]
mod dap_phase2_tests {
    use anyhow::Result;
    use perl_dap::breakpoints::BreakpointStore;
    use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
    use perl_dap::platform::normalize_path;
    use perl_dap::protocol::{SetBreakpointsArguments, Source, SourceBreakpoint};
    use perl_dap::{BridgeAdapter, create_attach_json_snippet, create_launch_json_snippet};
    use serde_json::{Value, json};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, channel};
    use std::time::{Duration, Instant};
    use tempfile::NamedTempFile;

    fn create_test_adapter() -> (DebugAdapter, Receiver<DapMessage>) {
        let (tx, rx) = channel();
        let mut adapter = DebugAdapter::new();
        adapter.set_event_sender(tx);
        (adapter, rx)
    }

    fn expect_response(msg: DapMessage, command: &str, expected_success: bool) -> Option<Value> {
        match msg {
            DapMessage::Response { success, command: c, body, .. } => {
                assert_eq!(c, command, "unexpected command");
                assert_eq!(success, expected_success, "unexpected success value");
                body
            }
            _ => panic!("expected response for command {command}"),
        }
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac5-adapter-scaffolding
    #[tokio::test]
    // AC:5
    async fn test_dap_adapter_scaffolding() -> Result<()> {
        let (mut adapter, rx) = create_test_adapter();

        let start = Instant::now();
        let init = adapter.handle_request(1, "initialize", None);
        let init_body = expect_response(init, "initialize", true)
            .ok_or_else(|| anyhow::anyhow!("initialize response should include capability body"))?;
        assert!(init_body.get("supportsConfigurationDoneRequest").is_some());
        assert!(start.elapsed() < Duration::from_millis(100), "initialize exceeded latency target");

        let initialized = rx.recv_timeout(Duration::from_millis(200))?;
        match initialized {
            DapMessage::Event { event, .. } => assert_eq!(event, "initialized"),
            _ => anyhow::bail!("expected initialized event"),
        }

        let disconnect = adapter.handle_request(2, "disconnect", None);
        let _ = expect_response(disconnect, "disconnect", true);
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac5-protocol-compliance
    #[tokio::test]
    // AC:5
    async fn test_json_rpc_protocol_compliance() -> Result<()> {
        let request =
            DapMessage::Request { seq: 7, command: "threads".to_string(), arguments: None };
        let serialized = serde_json::to_string(&request)?;
        assert!(serialized.contains("\"type\":\"request\""));
        assert!(serialized.contains("\"command\":\"threads\""));

        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(7, "threads", None);
        match response {
            DapMessage::Response { request_seq, command, success, .. } => {
                assert_eq!(request_seq, 7);
                assert_eq!(command, "threads");
                assert!(success);
            }
            _ => anyhow::bail!("expected threads response"),
        }

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac6-perl-shim-integration
    #[tokio::test]
    // AC:6
    async fn test_perl_shim_integration() -> Result<()> {
        // Bridge adapter remains available for Perl::LanguageServer-based workflows.
        let _bridge = BridgeAdapter::new();

        let attach_snippet = create_attach_json_snippet();
        let attach: Value = serde_json::from_str(&attach_snippet)?;
        assert_eq!(attach["request"], "attach");
        assert_eq!(attach["type"], "perl");
        assert!(attach.get("host").is_some());
        assert!(attach.get("port").is_some());

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac7-breakpoint-management
    #[tokio::test]
    // AC:7
    async fn test_breakpoint_management_with_ast_validation() -> Result<()> {
        let mut fixture = NamedTempFile::new()?;
        fixture.write_all(b"# comment line\nmy $x = 1;\nprint $x;\n")?;
        fixture.flush()?;
        let fixture_path = fixture.path().to_string_lossy().to_string();

        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(
            1,
            "setBreakpoints",
            Some(json!({
                "source": { "path": fixture_path },
                "breakpoints": [{ "line": 1 }, { "line": 2 }]
            })),
        );

        let body = expect_response(response, "setBreakpoints", true)
            .ok_or_else(|| anyhow::anyhow!("missing breakpoints response body"))?;
        let breakpoints = body
            .get("breakpoints")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("missing breakpoints array"))?;
        assert_eq!(breakpoints.len(), 2);
        assert!(
            !breakpoints[0].get("verified").and_then(Value::as_bool).unwrap_or(true),
            "comment line should not be verified"
        );
        assert!(
            breakpoints[1].get("verified").and_then(Value::as_bool).unwrap_or(false),
            "executable line should be verified"
        );

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac7-incremental-breakpoint-updates
    #[tokio::test]
    // AC:7
    async fn test_incremental_breakpoint_updates() -> Result<()> {
        let mut fixture = NamedTempFile::new()?;
        fixture.write_all(b"my $x = 1;\nmy $y = 2;\n")?;
        fixture.flush()?;
        let fixture_path = fixture.path().to_string_lossy().to_string();

        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source { path: Some(fixture_path.clone()), name: Some("test.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint { line: 2, column: None, condition: None }]),
            source_modified: None,
        };

        let initial = store.set_breakpoints(&args);
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].line, 2);

        store.adjust_breakpoints_for_edit(&fixture_path, 1, 3);
        let adjusted = store.get_breakpoints(&fixture_path);
        assert_eq!(adjusted.len(), 1);
        assert_eq!(adjusted[0].line, 5);

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac8-stack-and-variables
    #[tokio::test]
    // AC:8
    async fn test_stack_trace_and_scopes() -> Result<()> {
        let mut adapter = DebugAdapter::new();

        let stack = adapter.handle_request(1, "stackTrace", Some(json!({ "threadId": 1 })));
        let body = expect_response(stack, "stackTrace", true)
            .ok_or_else(|| anyhow::anyhow!("stackTrace body missing"))?;
        let stack_frames = body
            .get("stackFrames")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("stackFrames missing"))?;
        assert!(!stack_frames.is_empty(), "expected at least one stack frame");

        let scopes = adapter.handle_request(2, "scopes", Some(json!({ "frameId": 1 })));
        let scope_body = expect_response(scopes, "scopes", true)
            .ok_or_else(|| anyhow::anyhow!("scopes body missing"))?;
        let scope_list = scope_body
            .get("scopes")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("scopes array missing"))?;
        assert_eq!(scope_list.len(), 3, "expected locals/package/globals scopes");

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac8-lazy-variable-expansion
    #[tokio::test]
    // AC:8
    async fn test_lazy_variable_expansion() -> Result<()> {
        let mut adapter = DebugAdapter::new();
        let root = adapter.handle_request(
            1,
            "variables",
            Some(json!({
                "variablesReference": 11,
                "start": 0,
                "count": 20
            })),
        );
        let root_body = expect_response(root, "variables", true)
            .ok_or_else(|| anyhow::anyhow!("variables body missing"))?;
        let vars = root_body
            .get("variables")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("variables array missing"))?;
        assert!(!vars.is_empty());

        let child_ref = vars
            .iter()
            .find_map(|v| v.get("variablesReference").and_then(Value::as_i64))
            .unwrap_or(0);
        assert!(child_ref >= 0);

        if child_ref > 0 {
            let child = adapter.handle_request(
                2,
                "variables",
                Some(json!({
                    "variablesReference": child_ref,
                    "start": 0,
                    "count": 20
                })),
            );
            let _ = expect_response(child, "variables", true);
        }

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac9-execution-control
    #[tokio::test]
    // AC:9
    async fn test_execution_control_operations() -> Result<()> {
        let mut adapter = DebugAdapter::new();

        let cont = adapter.handle_request(1, "continue", Some(json!({ "threadId": 1 })));
        let _ = expect_response(cont, "continue", true);

        let next = adapter.handle_request(2, "next", Some(json!({ "threadId": 1 })));
        let _ = expect_response(next, "next", true);

        let step_in = adapter.handle_request(3, "stepIn", Some(json!({ "threadId": 1 })));
        let _ = expect_response(step_in, "stepIn", true);

        let step_out = adapter.handle_request(4, "stepOut", Some(json!({ "threadId": 1 })));
        let _ = expect_response(step_out, "stepOut", true);

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac9-pause-operation
    #[tokio::test]
    // AC:9
    async fn test_pause_interrupt_handling() -> Result<()> {
        let mut adapter = DebugAdapter::new();
        let pause = adapter.handle_request(1, "pause", Some(json!({ "threadId": 1 })));
        match pause {
            DapMessage::Response { success, command, message, .. } => {
                assert_eq!(command, "pause");
                assert!(!success, "pause should fail when no active debug session exists");
                let msg = message.ok_or_else(|| anyhow::anyhow!("pause should return message"))?;
                assert!(msg.contains("Failed to pause debugger"));
            }
            _ => anyhow::bail!("expected pause response"),
        }
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac10-evaluate-and-repl
    #[tokio::test]
    // AC:10
    async fn test_evaluate_in_frame_context() -> Result<()> {
        let mut adapter = DebugAdapter::new();
        let resp = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({
                "expression": "$x + 1",
                "frameId": 1
            })),
        );

        match resp {
            DapMessage::Response { success, command, message, .. } => {
                assert_eq!(command, "evaluate");
                assert!(!success, "evaluate should fail without session");
                let msg = message.ok_or_else(|| anyhow::anyhow!("missing evaluate error"))?;
                assert!(msg.contains("No debugger session"));
            }
            _ => anyhow::bail!("expected evaluate response"),
        }
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac10-safe-evaluation
    #[tokio::test]
    // AC:10
    async fn test_safe_evaluation_mode() -> Result<()> {
        let mut adapter = DebugAdapter::new();

        let blocked = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({
                "expression": "system('rm -rf /')"
            })),
        );
        match blocked {
            DapMessage::Response { success, message, .. } => {
                assert!(!success);
                let msg = message.ok_or_else(|| anyhow::anyhow!("missing blocked message"))?;
                assert!(msg.contains("Safe evaluation mode"));
            }
            _ => anyhow::bail!("expected evaluate response"),
        }

        // Side effects opt-in bypasses safety validator but still requires an active session.
        let no_session = adapter.handle_request(
            2,
            "evaluate",
            Some(json!({
                "expression": "system('echo ok')",
                "allowSideEffects": true
            })),
        );
        match no_session {
            DapMessage::Response { success, message, .. } => {
                assert!(!success);
                let msg = message.ok_or_else(|| anyhow::anyhow!("missing no-session message"))?;
                assert!(msg.contains("No debugger session"));
            }
            _ => anyhow::bail!("expected evaluate response"),
        }

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac11-vscode-integration
    #[test]
    // AC:11
    fn test_vscode_native_integration() -> Result<()> {
        let launch_json: Value = serde_json::from_str(&create_launch_json_snippet())?;
        assert_eq!(launch_json["request"], "launch");
        assert_eq!(launch_json["type"], "perl");

        let attach_json: Value = serde_json::from_str(&create_attach_json_snippet())?;
        assert_eq!(attach_json["request"], "attach");
        assert_eq!(attach_json["type"], "perl");

        let extension_manifest =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vscode-extension/package.json");
        let manifest: Value = serde_json::from_str(&std::fs::read_to_string(extension_manifest)?)?;
        let debuggers = manifest["contributes"]["debuggers"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing contributes.debuggers"))?;
        assert!(debuggers.iter().any(|dbg| dbg["type"] == "perl"));

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac12-cross-platform-wsl
    #[tokio::test]
    // AC:12
    async fn test_cross_platform_wsl_support() -> Result<()> {
        let unix = normalize_path(PathBuf::from("/tmp/example.pl").as_path());
        assert!(!unix.as_os_str().is_empty());

        let wsl = normalize_path(PathBuf::from("/mnt/c/Users/test/script.pl").as_path());
        assert!(!wsl.as_os_str().is_empty());

        #[cfg(target_os = "linux")]
        {
            let s = wsl.to_string_lossy();
            assert!(
                s.starts_with("C:"),
                "expected /mnt/c path to translate to Windows-style drive path, got {s}"
            );
        }

        Ok(())
    }
}
