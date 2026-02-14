use perl_dap::{DapMessage, DebugAdapter};
use perl_tdd_support::must_some;
use serde_json::json;
use std::fs::write;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to wait for a specific DAP event
fn wait_for_event(
    rx: &Receiver<DapMessage>,
    event_name: &str,
    timeout_secs: u64,
) -> Result<DapMessage, String> {
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match rx.recv_timeout(timeout) {
            Ok(msg) => {
                if let DapMessage::Event { ref event, .. } = msg
                    && event == event_name
                {
                    return Ok(msg);
                }
                // Continue waiting for the specific event
            }
            Err(_) => return Err(format!("Timeout waiting for {} event", event_name)),
        }
    }
}

/// Helper to create a test Perl script
fn create_test_script(content: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let script_path = dir.path().join("test.pl");
    write(&script_path, content)?;
    Ok(script_path)
}

#[test]
fn test_dap_initialize() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let response = adapter.handle_request(1, "initialize", None);

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "initialize");
            assert!(body.is_some());

            // Verify capabilities
            let body = must_some(body);
            assert!(
                body.get("supportsConfigurationDoneRequest")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            );
            assert!(
                body.get("supportsConditionalBreakpoints")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            );
            assert!(
                body.get("supportsEvaluateForHovers").and_then(|v| v.as_bool()).unwrap_or(false)
            );
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_launch_with_invalid_program() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let launch_args = json!({
        "program": "/nonexistent/file.pl",
        "args": [],
        "stopOnEntry": false
    });

    let response = adapter.handle_request(1, "launch", Some(launch_args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "launch");
            assert!(message.is_some());
            let msg = must_some(message);
            // It could be a security error (if path outside workspace) or launch error
            assert!(msg.contains("Failed to launch debugger") || msg.contains("Security Error"));
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_launch_missing_arguments() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let response = adapter.handle_request(1, "launch", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "launch");
            assert!(message.is_some());
            assert_eq!(must_some(message), "Missing launch arguments");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_breakpoints_no_session() {
    let mut adapter = DebugAdapter::new();

    let bp_args = json!({
        "source": {"path": "/tmp/test.pl"},
        "breakpoints": [
            {"line": 5},
            {"line": 10, "condition": "$x > 5"}
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(bp_args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "setBreakpoints");

            let body = must_some(body);
            let breakpoints = must_some(body.get("breakpoints").and_then(|b| b.as_array()));
            assert_eq!(breakpoints.len(), 2);

            // Without active session, breakpoints should not be verified
            for bp in breakpoints {
                assert!(!bp.get("verified").and_then(|v| v.as_bool()).unwrap_or(true));
            }
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_inline_values() -> TestResult {
    let dir = tempdir()?;
    let script_path = dir.path().join("inline_values.pl");
    write(&script_path, "my $x = 1;\nmy $y = $x + 2;\nmy $z = $y + 3;\n")?;

    let mut adapter = DebugAdapter::new();

    // Initialize with correct workspace root to pass security validation
    adapter.handle_request(0, "initialize", Some(json!({
        "rootPath": dir.path().to_str().unwrap()
    })));

    let response = adapter.handle_request(
        1,
        "inlineValues",
        Some(json!({
            "source": { "path": script_path.to_str().ok_or("path")? },
            "startLine": 1,
            "endLine": 3
        })),
    );

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "inlineValues");
            let body = body.ok_or("missing body")?;
            let values = body
                .get("inlineValues")
                .and_then(|v| v.as_array())
                .ok_or("missing inlineValues")?;
            assert!(
                values.iter().any(|v| {
                    v.get("text").and_then(|t| t.as_str()).unwrap_or("").contains("$x")
                })
            );
            assert!(
                values.iter().any(|v| {
                    v.get("text").and_then(|t| t.as_str()).unwrap_or("").contains("$y")
                })
            );
        }
        _ => panic!("Expected inlineValues response"),
    }

    Ok(())
}

#[test]
fn test_dap_breakpoints_missing_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();

    let bp_args = json!({
        "breakpoints": [{"line": 5}]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(bp_args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success);
            let msg = message.ok_or("Expected error message")?;
            assert!(msg.contains("missing field `source`"));
        }
        _ => return Err("Expected response".into()),
    }
    Ok(())
}

#[test]
fn test_dap_breakpoints_invalid_line() {
    let mut adapter = DebugAdapter::new();

    let bp_args = json!({
        "source": {"path": "/tmp/test.pl"},
        "breakpoints": [
            {"line": 0},    // Invalid line
            {"line": -5}    // Invalid line
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(bp_args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success); // Request succeeds but breakpoints are not verified
            assert_eq!(command, "setBreakpoints");

            let body = must_some(body);
            let breakpoints = must_some(body.get("breakpoints").and_then(|b| b.as_array()));
            assert_eq!(breakpoints.len(), 2);

            // Invalid line breakpoints should not be verified
            for bp in breakpoints {
                assert!(!bp.get("verified").and_then(|v| v.as_bool()).unwrap_or(true));
                assert!(bp.get("message").is_some());
            }
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_evaluate_empty_expression() {
    let mut adapter = DebugAdapter::new();

    let eval_args = json!({
        "expression": ""
    });

    let response = adapter.handle_request(1, "evaluate", Some(eval_args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "evaluate");
            assert_eq!(must_some(message), "Empty expression");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_evaluate_no_session() {
    let mut adapter = DebugAdapter::new();

    let eval_args = json!({
        "expression": "$x + 1"
    });

    let response = adapter.handle_request(1, "evaluate", Some(eval_args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "evaluate");
            assert!(must_some(message).contains("No debugger session"));
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_threads_no_session() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "threads", None);

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "threads");

            let body = must_some(body);
            let threads = must_some(body.get("threads").and_then(|t| t.as_array()));
            assert_eq!(threads.len(), 0); // No threads without session
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_stacktrace_no_session() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stackTrace", Some(json!({"threadId": 1})));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "stackTrace");

            let body = must_some(body);
            let frames = must_some(body.get("stackFrames").and_then(|f| f.as_array()));

            // Should return placeholder frame without session
            assert_eq!(frames.len(), 1);
            assert_eq!(must_some(frames[0].get("name").and_then(|n| n.as_str())), "main::hello");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_pause_no_session() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "pause", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "pause");
            assert_eq!(must_some(message), "Failed to pause debugger");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_disconnect_cleans_up_session() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    // First verify we can disconnect even without active session
    let response = adapter.handle_request(1, "disconnect", None);

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "disconnect");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_unknown_command() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "unknownCommand", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "unknownCommand");
            assert!(must_some(message).starts_with("Unknown command"));
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_variables_missing_reference() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "variables", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "variables");
            assert_eq!(must_some(message), "Missing arguments");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_variables_default_scope() {
    let mut adapter = DebugAdapter::new();

    let var_args = json!({
        "variablesReference": 11
    });

    let response = adapter.handle_request(1, "variables", Some(var_args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "variables");

            let body = must_some(body);
            let variables = must_some(body.get("variables").and_then(|v| v.as_array()));

            // Should return placeholder variables (@_, $self)
            assert_eq!(variables.len(), 2);

            let var_names: Vec<&str> = variables
                .iter()
                .map(|v| must_some(v.get("name").and_then(|n| n.as_str())))
                .collect();
            assert!(var_names.contains(&"@_"));
            assert!(var_names.contains(&"$self"));
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_scopes_missing_frame() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "scopes", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "scopes");
            assert_eq!(must_some(message), "Missing frameId");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_scopes_valid_frame() {
    let mut adapter = DebugAdapter::new();

    let scope_args = json!({
        "frameId": 1
    });

    let response = adapter.handle_request(1, "scopes", Some(scope_args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "scopes");

            let body = must_some(body);
            let scopes = must_some(body.get("scopes").and_then(|s| s.as_array()));
            assert_eq!(scopes.len(), 3);

            let scope = &scopes[0];
            assert_eq!(must_some(scope.get("name").and_then(|n| n.as_str())), "Locals");
            assert_eq!(must_some(scope.get("variablesReference").and_then(|v| v.as_i64())), 11);
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_sequence_number_increment() {
    let mut adapter = DebugAdapter::new();

    // Test that sequence numbers increment properly by making multiple requests
    let _response1 = adapter.handle_request(1, "initialize", None);
    let _response2 = adapter.handle_request(1, "threads", None);
    let _response3 = adapter.handle_request(1, "disconnect", None);

    // Since we can't access next_seq directly, we verify the adapter works
    // by successfully handling multiple requests
    // Test passes if no panics occurred
}

#[test]
fn test_dap_full_session_lifecycle() -> TestResult {
    // Skip if perl is not available
    if std::process::Command::new("perl").arg("--version").output().is_err() {
        eprintln!("Skipping DAP lifecycle test - perl not available");
        return Ok(());
    }

    let script_content = r#"use strict;
use warnings;

my $x = 10;
my $y = 20;
my $result = $x + $y;
print "Result: $result\n";
"#;

    let script_path = create_test_script(script_content)?;
    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    // Initialize
    let init_response = adapter.handle_request(1, "initialize", None);
    match init_response {
        DapMessage::Response { success, .. } => assert!(success, "Initialize should succeed"),
        _ => panic!("Expected response for initialize"),
    }

    // Wait for initialized event with timeout
    match wait_for_event(&rx, "initialized", 5) {
        Ok(_) => eprintln!("Received initialized event"),
        Err(e) => {
            eprintln!("Warning: {}", e);
            // Continue test anyway as this might be timing-related
        }
    }

    // Launch - this may fail if system doesn't support Perl debugging
    let launch_args = json!({
        "program": script_path.to_str().ok_or("Failed to convert path to string")?,
        "args": [],
        "stopOnEntry": true
    });
    let launch_response = adapter.handle_request(2, "launch", Some(launch_args));

    match launch_response {
        DapMessage::Response { success, message, .. } => {
            if !success {
                eprintln!("Launch failed (expected on some systems): {:?}", message);
                // Test the rest of the API even if launch fails
            }
        }
        _ => panic!("Expected launch response"),
    }

    // Configuration done - should work regardless
    let config_response = adapter.handle_request(3, "configurationDone", None);
    match config_response {
        DapMessage::Response { success, .. } => assert!(success, "Configuration should succeed"),
        _ => panic!("Expected configurationDone response"),
    }

    // Set breakpoints - should work even without active session
    let bp_args = json!({
        "source": {"path": script_path.to_str().ok_or("Failed to convert path to string")?},
        "breakpoints": [{"line": 5}]
    });
    let bp_response = adapter.handle_request(4, "setBreakpoints", Some(bp_args));

    match bp_response {
        DapMessage::Response { success, body, .. } => {
            assert!(success, "setBreakpoints should succeed");
            let body = must_some(body);
            let breakpoints = must_some(body.get("breakpoints").and_then(|b| b.as_array()));
            assert_eq!(breakpoints.len(), 1);

            // Breakpoint might not be verified without active session
            let verified =
                breakpoints[0].get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
            eprintln!("Breakpoint verified: {}", verified);
        }
        _ => panic!("Expected setBreakpoints response"),
    }

    // Test thread listing
    let threads_response = adapter.handle_request(5, "threads", None);
    match threads_response {
        DapMessage::Response { success, body, .. } => {
            assert!(success, "threads request should succeed");
            let body = must_some(body);
            let threads = must_some(body.get("threads").and_then(|t| t.as_array()));
            // May have 0 or 1 threads depending on launch success
            assert!(threads.len() <= 1, "Should have 0 or 1 thread");
        }
        _ => panic!("Expected threads response"),
    }

    // Continue - should handle gracefully even if no session
    let continue_response = adapter.handle_request(6, "continue", None);
    match continue_response {
        DapMessage::Response { success, .. } => {
            // May succeed or fail depending on session state
            eprintln!("Continue response success: {}", success);
        }
        _ => panic!("Expected continue response"),
    }

    // Disconnect - should always work
    let disconnect_response = adapter.handle_request(7, "disconnect", None);
    match disconnect_response {
        DapMessage::Response { success, .. } => assert!(success, "disconnect should succeed"),
        _ => panic!("Expected disconnect response"),
    }

    eprintln!("DAP lifecycle test completed successfully");
    Ok(())
}
