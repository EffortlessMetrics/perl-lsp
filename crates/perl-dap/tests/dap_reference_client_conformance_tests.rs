//! Reference-client-style DAP conformance checks.
//!
//! These tests mimic how a real DAP client probes an adapter surface: send
//! requests across the command matrix and assert that each response is a
//! well-formed DAP response envelope with matching metadata.

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct CommandProbe {
    command: &'static str,
    arguments: Option<Value>,
}

fn command_probes() -> Vec<CommandProbe> {
    vec![
        CommandProbe { command: "initialize", arguments: None },
        CommandProbe { command: "launch", arguments: None },
        CommandProbe { command: "attach", arguments: None },
        CommandProbe {
            command: "disconnect",
            arguments: Some(json!({"terminateDebuggee": false})),
        },
        CommandProbe { command: "terminate", arguments: Some(json!({})) },
        CommandProbe {
            command: "setBreakpoints",
            arguments: Some(json!({
                "source": {"path": "/tmp/reference-client.pl"},
                "breakpoints": [{"line": 1}]
            })),
        },
        CommandProbe {
            command: "setFunctionBreakpoints",
            arguments: Some(json!({"breakpoints": [{"name": "main"}]})),
        },
        CommandProbe {
            command: "setExceptionBreakpoints",
            arguments: Some(json!({"filters": ["all"]})),
        },
        CommandProbe { command: "configurationDone", arguments: None },
        CommandProbe { command: "threads", arguments: None },
        CommandProbe { command: "stackTrace", arguments: Some(json!({"threadId": 1})) },
        CommandProbe { command: "scopes", arguments: Some(json!({"frameId": 1})) },
        CommandProbe { command: "variables", arguments: Some(json!({"variablesReference": 0})) },
        CommandProbe {
            command: "setVariable",
            arguments: Some(json!({"variablesReference": 0, "name": "$x", "value": "1"})),
        },
        CommandProbe { command: "continue", arguments: Some(json!({"threadId": 1})) },
        CommandProbe { command: "next", arguments: Some(json!({"threadId": 1})) },
        CommandProbe { command: "stepIn", arguments: Some(json!({"threadId": 1})) },
        CommandProbe { command: "stepOut", arguments: Some(json!({"threadId": 1})) },
        CommandProbe { command: "pause", arguments: Some(json!({"threadId": 1})) },
        CommandProbe { command: "evaluate", arguments: Some(json!({"expression": "$x"})) },
        CommandProbe {
            command: "inlineValues",
            arguments: Some(json!({
                "source": {"path": "/tmp/reference-client.pl"},
                "startLine": 1,
                "endLine": 1
            })),
        },
        CommandProbe {
            command: "breakpointLocations",
            arguments: Some(json!({
                "source": {"path": "/tmp/reference-client.pl"},
                "line": 1
            })),
        },
        CommandProbe { command: "source", arguments: Some(json!({"sourceReference": 1})) },
        CommandProbe { command: "loadedSources", arguments: None },
        CommandProbe { command: "modules", arguments: Some(json!({})) },
        CommandProbe {
            command: "completions",
            arguments: Some(json!({"text": "$", "column": 1, "line": 1})),
        },
        CommandProbe { command: "exceptionInfo", arguments: None },
        CommandProbe { command: "restart", arguments: Some(json!({})) },
        CommandProbe {
            command: "setExpression",
            arguments: Some(json!({"expression": "$x", "value": "42"})),
        },
        CommandProbe { command: "dataBreakpointInfo", arguments: Some(json!({"name": "$x"})) },
        CommandProbe { command: "setDataBreakpoints", arguments: Some(json!({"breakpoints": []})) },
        CommandProbe { command: "cancel", arguments: Some(json!({"requestId": 1})) },
        CommandProbe { command: "stepInTargets", arguments: Some(json!({"frameId": 1})) },
        CommandProbe {
            command: "gotoTargets",
            arguments: Some(json!({
                "source": {"path": "/tmp/reference-client.pl"},
                "line": 1
            })),
        },
        CommandProbe { command: "goto", arguments: Some(json!({"threadId": 1, "targetId": 9999})) },
        CommandProbe { command: "restartFrame", arguments: Some(json!({"frameId": 1})) },
        CommandProbe { command: "terminateThreads", arguments: Some(json!({"threadIds": [1]})) },
    ]
}

#[test]
fn dap_surface_returns_well_formed_response_envelopes() -> TestResult {
    for (index, probe) in command_probes().iter().enumerate() {
        let mut adapter = DebugAdapter::new();
        let request_seq = 10_000_i64 + index as i64;
        let response = adapter.handle_request(request_seq, probe.command, probe.arguments.clone());

        match response {
            DapMessage::Response { seq, request_seq: echoed_request_seq, command, .. } => {
                assert!(seq >= 1, "seq must be positive for command `{}`", probe.command);
                assert_eq!(echoed_request_seq, request_seq, "request_seq echo mismatch");
                assert_eq!(command, probe.command, "command echo mismatch");
            }
            other => {
                return Err(format!(
                    "expected Response envelope for `{}`, got {:?}",
                    probe.command, other
                )
                .into());
            }
        }
    }

    Ok(())
}

#[test]
fn initialize_reports_capability_shape_expected_by_reference_clients() -> TestResult {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(1, "initialize", None);

    let body = match response {
        DapMessage::Response { success: true, command, body, .. } => {
            assert_eq!(command, "initialize");
            body.ok_or("initialize response missing body")?
        }
        DapMessage::Response { success: false, message, .. } => {
            return Err(format!(
                "initialize unexpectedly failed: {}",
                message.unwrap_or_else(|| "<no message>".to_string())
            )
            .into());
        }
        other => return Err(format!("expected response for initialize, got {other:?}").into()),
    };

    for flag in [
        "supportsConfigurationDoneRequest",
        "supportsFunctionBreakpoints",
        "supportsConditionalBreakpoints",
        "supportsEvaluateForHovers",
        "supportsSetVariable",
        "supportsCompletionsRequest",
    ] {
        assert_eq!(
            body.get(flag).and_then(Value::as_bool),
            Some(true),
            "initialize capability `{flag}` should be true"
        );
    }

    Ok(())
}
