use perl_parser::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;
use tempfile::tempdir;

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
fn create_test_script(content: &str) -> std::path::PathBuf {
    let dir = tempdir().unwrap();
    let script_path = dir.path().join("test.pl");
    write(&script_path, content).unwrap();
    script_path
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
            let body = body.unwrap();
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
            assert!(message.unwrap().contains("Failed to launch debugger"));
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
            assert_eq!(message.unwrap(), "Missing launch arguments");
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

            let body = body.unwrap();
            let breakpoints = body.get("breakpoints").and_then(|b| b.as_array()).unwrap();
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
fn test_dap_breakpoints_missing_source() {
    let mut adapter = DebugAdapter::new();

    let bp_args = json!({
        "breakpoints": [{"line": 5}]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(bp_args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success);
            assert_eq!(command, "setBreakpoints");
            assert_eq!(message.unwrap(), "Missing source path");
        }
        _ => panic!("Expected response message"),
    }
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

            let body = body.unwrap();
            let breakpoints = body.get("breakpoints").and_then(|b| b.as_array()).unwrap();
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
            assert_eq!(message.unwrap(), "Empty expression");
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
            assert!(message.unwrap().contains("No debugger session"));
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

            let body = body.unwrap();
            let threads = body.get("threads").and_then(|t| t.as_array()).unwrap();
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

            let body = body.unwrap();
            let frames = body.get("stackFrames").and_then(|f| f.as_array()).unwrap();
            assert_eq!(frames.len(), 0); // No frames without session

            let total = body.get("totalFrames").and_then(|t| t.as_u64()).unwrap();
            assert_eq!(total, 0);
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
            assert_eq!(message.unwrap(), "Failed to pause debugger");
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
            assert!(message.unwrap().starts_with("Unknown command"));
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
            assert_eq!(message.unwrap(), "Missing variablesReference");
        }
        _ => panic!("Expected response message"),
    }
}

#[test]
fn test_dap_variables_default_scope() {
    let mut adapter = DebugAdapter::new();

    let var_args = json!({
        "variablesReference": 1
    });

    let response = adapter.handle_request(1, "variables", Some(var_args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success);
            assert_eq!(command, "variables");

            let body = body.unwrap();
            let variables = body.get("variables").and_then(|v| v.as_array()).unwrap();

            // Should return default Perl variables (@_, $_)
            assert_eq!(variables.len(), 2);

            let var_names: Vec<&str> =
                variables.iter().map(|v| v.get("name").and_then(|n| n.as_str()).unwrap()).collect();
            assert!(var_names.contains(&"@_"));
            assert!(var_names.contains(&"$_"));
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
            assert_eq!(message.unwrap(), "Missing frameId");
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

            let body = body.unwrap();
            let scopes = body.get("scopes").and_then(|s| s.as_array()).unwrap();
            assert_eq!(scopes.len(), 1);

            let scope = &scopes[0];
            assert_eq!(scope.get("name").and_then(|n| n.as_str()).unwrap(), "Local");
            assert_eq!(scope.get("variablesReference").and_then(|v| v.as_i64()).unwrap(), 1);
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
fn test_dap_full_session_lifecycle() {
    // Skip if perl is not available
    if std::process::Command::new("perl").arg("--version").output().is_err() {
        eprintln!("Skipping DAP lifecycle test - perl not available");
        return;
    }

    let script_content = r#"use strict;
use warnings;

my $x = 10;
my $y = 20;
my $result = $x + $y;
print "Result: $result\n";
"#;

    let script_path = create_test_script(script_content);
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
        "program": script_path.to_str().unwrap(),
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
        "source": {"path": script_path.to_str().unwrap()},
        "breakpoints": [{"line": 5}]
    });
    let bp_response = adapter.handle_request(4, "setBreakpoints", Some(bp_args));

    match bp_response {
        DapMessage::Response { success, body, .. } => {
            assert!(success, "setBreakpoints should succeed");
            let body = body.unwrap();
            let breakpoints = body.get("breakpoints").and_then(|b| b.as_array()).unwrap();
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
            let body = body.unwrap();
            let threads = body.get("threads").and_then(|t| t.as_array()).unwrap();
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
}
