//! DAP Session Lifecycle Tests (AC5.5)
//!
//! Comprehensive tests for DAP session management including:
//! - Session state transitions (initialize → launch/attach → disconnect)
//! - JSON-RPC 2.0 protocol compliance (AC5.1)
//! - Thread-safe session state management (AC5.3)
//! - Error handling with anyhow::Result (AC5.4)
//!
//! Specification: GitHub Issue #449 - AC5.1, AC5.3, AC5.4, AC5.5

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use perl_tdd_support::{must, must_some};
use serde_json::json;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Helper to create a test adapter with message capture
fn create_test_adapter() -> (DebugAdapter, Receiver<DapMessage>) {
    let (tx, rx) = channel();
    let mut adapter = DebugAdapter::new();
    adapter.set_event_sender(tx);
    (adapter, rx)
}

/// Helper to wait for events with timeout
fn wait_for_event(rx: &Receiver<DapMessage>, timeout_ms: u64) -> Option<DapMessage> {
    rx.recv_timeout(Duration::from_millis(timeout_ms)).ok()
}

// ============================================================================
// AC5.5: Session Lifecycle Tests
// ============================================================================

#[test]
// AC:5.5
fn test_session_lifecycle_initialize() {
    // Test that initialize request returns capabilities and emits initialized event
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "initialize", None);

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "Initialize should succeed");
            assert_eq!(command, "initialize");
            assert!(body.is_some(), "Initialize should return capabilities");

            // Verify capabilities structure
            let caps = must_some(body);
            assert!(caps.get("supportsConfigurationDoneRequest").is_some());
            assert!(caps.get("supportsEvaluateForHovers").is_some());
            assert!(caps.get("supportsInlineValues").is_some());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }

    // Verify initialized event is sent
    let event = wait_for_event(&rx, 100);
    assert!(event.is_some(), "Should emit initialized event");
    match must_some(event) {
        DapMessage::Event { event, .. } => {
            assert_eq!(event, "initialized");
        }
        _ => must(Err::<(), _>(format!("Expected Event message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_disconnect_without_session() {
    // Test that disconnect works even without an active session
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "disconnect", None);

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "Disconnect should succeed even without session");
            assert_eq!(command, "disconnect");
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }

    // Should emit terminated event
    let event = wait_for_event(&rx, 100);
    assert!(event.is_some(), "Should emit terminated event");
    match must_some(event) {
        DapMessage::Event { event, .. } => {
            assert_eq!(event, "terminated");
        }
        _ => must(Err::<(), _>(format!("Expected Event message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_terminate_without_session() {
    // Test that terminate works even without an active session and emits terminated event
    let (mut adapter, rx) = create_test_adapter();

    let response = adapter.handle_request(1, "terminate", Some(json!({ "restart": false })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "Terminate should succeed even without session");
            assert_eq!(command, "terminate");
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }

    let event = wait_for_event(&rx, 100);
    assert!(event.is_some(), "Should emit terminated event");
    match must_some(event) {
        DapMessage::Event { event, body, .. } => {
            assert_eq!(event, "terminated");
            let restart = body
                .as_ref()
                .and_then(|value| value.get("restart"))
                .and_then(|value| value.as_bool());
            assert_eq!(restart, Some(false), "terminate event should include restart flag");
        }
        _ => must(Err::<(), _>(format!("Expected Event message"))),
    }
}

#[test]
// AC:5.5
fn test_set_variable_without_session_returns_error() {
    // setVariable should fail clearly when no debugger session is active
    let (mut adapter, _rx) = create_test_adapter();

    let response = adapter.handle_request(
        1,
        "setVariable",
        Some(json!({
            "variablesReference": 11,
            "name": "$x",
            "value": "2"
        })),
    );

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "setVariable should fail without active session");
            assert_eq!(command, "setVariable");
            let msg = must_some(message);
            assert!(
                msg.contains("No debugger session"),
                "Error should mention missing session: {}",
                msg
            );
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_set_variable_rejects_invalid_variable_name() {
    // setVariable should reject names that could inject debugger commands
    let (mut adapter, _rx) = create_test_adapter();

    let response = adapter.handle_request(
        1,
        "setVariable",
        Some(json!({
            "variablesReference": 11,
            "name": "$x; system('id')",
            "value": "2"
        })),
    );

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "setVariable should reject invalid variable names");
            assert_eq!(command, "setVariable");
            let msg = must_some(message);
            assert!(
                msg.contains("Invalid variable name"),
                "Error should report invalid variable name: {}",
                msg
            );
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_launch_missing_arguments() {
    // Test that launch fails gracefully with missing arguments
    let (mut adapter, _rx) = create_test_adapter();

    let response = adapter.handle_request(1, "launch", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "Launch should fail without arguments");
            assert_eq!(command, "launch");
            assert!(message.is_some());
            assert!(must_some(message).contains("Missing launch arguments"));
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_launch_empty_program() {
    // Test that launch fails with empty program path
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "program": "",
        "args": [],
        "stopOnEntry": false
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "Launch should fail with empty program");
            assert_eq!(command, "launch");
            assert!(message.is_some());
            let msg = must_some(message);
            assert!(
                msg.contains("empty") || msg.contains("cannot be empty"),
                "Error should mention empty path: {}",
                msg
            );
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_launch_nonexistent_program() {
    // Test that launch fails with non-existent program path
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "program": "/nonexistent/path/to/script.pl",
        "args": [],
        "stopOnEntry": false
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "Launch should fail with nonexistent program");
            assert_eq!(command, "launch");
            assert!(message.is_some());
            let msg = must_some(message);
            assert!(
                msg.contains("Could not access") || msg.contains("not a regular file"),
                "Error should mention access issue: {}",
                msg
            );
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_attach_missing_arguments() {
    // Test that attach fails gracefully with missing arguments
    let (mut adapter, _rx) = create_test_adapter();

    let response = adapter.handle_request(1, "attach", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "Attach should fail without arguments");
            assert_eq!(command, "attach");
            assert!(message.is_some());
            assert!(must_some(message).contains("Missing attach arguments"));
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_attach_validation() {
    // Test that attach validates arguments correctly
    let (mut adapter, _rx) = create_test_adapter();

    // Test with valid TCP attach arguments
    let args = json!({
        "host": "localhost",
        "port": 13603,
        "timeout": 5000
    });

    let response = adapter.handle_request_mock(1, "attach", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "Attach not yet implemented");
            assert_eq!(command, "attach");
            assert!(message.is_some());
            // Should validate but indicate not implemented
            let msg = must_some(message);
            assert!(
                msg.contains("not yet fully implemented")
                    || msg.contains("localhost:13603")
                    || msg.contains("Process ID attachment"),
                "Should validate args: {}",
                msg
            );
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_state_transitions() {
    // Test proper state transitions through session lifecycle
    let (mut adapter, rx) = create_test_adapter();

    // 1. Initialize
    let init_response = adapter.handle_request(1, "initialize", None);
    match init_response {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }
    let init_event = wait_for_event(&rx, 100);
    assert!(init_event.is_some());

    // 2. Set breakpoints (before launch)
    let bp_args = json!({
        "source": { "path": "/test.pl" },
        "breakpoints": [{ "line": 10 }]
    });
    let bp_response = adapter.handle_request(2, "setBreakpoints", Some(bp_args));
    match bp_response {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }

    // 3. Configuration done
    let config_response = adapter.handle_request(3, "configurationDone", None);
    match config_response {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }

    // 4. Disconnect
    let disconnect_response = adapter.handle_request(4, "disconnect", None);
    match disconnect_response {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_threads_request() {
    // Test threads request returns proper structure
    let (mut adapter, _rx) = create_test_adapter();

    let response = adapter.handle_request(1, "threads", None);

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "Threads request should succeed");
            assert_eq!(command, "threads");
            assert!(body.is_some());

            // Even without active session, should return empty threads array
            let body_val = must_some(body);
            assert!(body_val.get("threads").is_some());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_stacktrace_request() {
    // Test stackTrace request returns proper structure
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "stackTrace", Some(args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "StackTrace request should succeed");
            assert_eq!(command, "stackTrace");
            assert!(body.is_some());

            // Should return empty frames without active session
            let body_val = must_some(body);
            assert!(body_val.get("stackFrames").is_some());
            assert!(body_val.get("totalFrames").is_some());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_scopes_request() {
    // Test scopes request returns proper structure
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "frameId": 1
    });

    let response = adapter.handle_request(1, "scopes", Some(args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "Scopes request should succeed");
            assert_eq!(command, "scopes");
            assert!(body.is_some());

            let body_val = must_some(body);
            assert!(body_val.get("scopes").is_some());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.5
fn test_session_lifecycle_variables_request() {
    // Test variables request returns proper structure
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "variablesReference": 1
    });

    let response = adapter.handle_request(1, "variables", Some(args));

    match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "Variables request should succeed");
            assert_eq!(command, "variables");
            assert!(body.is_some());

            let body_val = must_some(body);
            assert!(body_val.get("variables").is_some());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

// ============================================================================
// AC5.1: JSON-RPC 2.0 Message Framing Tests
// ============================================================================

#[test]
// AC:5.1
fn test_message_framing_request_structure() {
    // Test that DapMessage::Request has proper structure
    let request =
        DapMessage::Request { seq: 1, command: "initialize".to_string(), arguments: None };

    // Serialize to JSON
    let json = must(serde_json::to_string(&request));

    // Verify structure
    let parsed: serde_json::Value = must(serde_json::from_str(&json));
    assert_eq!(parsed.get("type").and_then(|v| v.as_str()), Some("request"));
    assert_eq!(parsed.get("seq").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(parsed.get("command").and_then(|v| v.as_str()), Some("initialize"));
}

#[test]
// AC:5.1
fn test_message_framing_response_structure() {
    // Test that DapMessage::Response has proper structure
    let response = DapMessage::Response {
        seq: 2,
        request_seq: 1,
        success: true,
        command: "initialize".to_string(),
        body: Some(json!({"supportsConfigurationDoneRequest": true})),
        message: None,
    };

    // Serialize to JSON
    let json = must(serde_json::to_string(&response));

    // Verify structure
    let parsed: serde_json::Value = must(serde_json::from_str(&json));
    assert_eq!(parsed.get("type").and_then(|v| v.as_str()), Some("response"));
    assert_eq!(parsed.get("seq").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(parsed.get("request_seq").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(true));
    assert!(parsed.get("body").is_some());
}

#[test]
// AC:5.1
fn test_message_framing_event_structure() {
    // Test that DapMessage::Event has proper structure
    let event = DapMessage::Event { seq: 3, event: "initialized".to_string(), body: None };

    // Serialize to JSON
    let json = must(serde_json::to_string(&event));

    // Verify structure
    let parsed: serde_json::Value = must(serde_json::from_str(&json));
    assert_eq!(parsed.get("type").and_then(|v| v.as_str()), Some("event"));
    assert_eq!(parsed.get("seq").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(parsed.get("event").and_then(|v| v.as_str()), Some("initialized"));
}

#[test]
// AC:5.1
fn test_message_framing_content_length() {
    // Test that messages can be properly framed with Content-Length
    let response = DapMessage::Response {
        seq: 1,
        request_seq: 1,
        success: true,
        command: "initialize".to_string(),
        body: None,
        message: None,
    };

    let json = must(serde_json::to_string(&response));
    let content_length = json.len();
    let frame = format!("Content-Length: {}\r\n\r\n{}", content_length, json);

    // Verify frame structure
    assert!(frame.starts_with("Content-Length: "));
    assert!(frame.contains("\r\n\r\n"));
    assert!(frame.contains(r#""type":"response""#));
}

// ============================================================================
// AC5.3: Thread-Safe Architecture Tests
// ============================================================================

#[test]
// AC:5.3
fn test_thread_safe_sequence_numbers() {
    // Test that sequence numbers are thread-safe and monotonically increasing
    let adapter = Arc::new(Mutex::new(DebugAdapter::new()));
    let mut handles = vec![];

    // Spawn multiple threads making requests
    for i in 0..10 {
        let adapter_clone = Arc::clone(&adapter);
        let handle = thread::spawn(move || {
            let mut adapter = must(adapter_clone.lock());
            let response = adapter.handle_request(i, "initialize", None);
            match response {
                DapMessage::Response { seq, .. } => seq,
                _ => {
                    must(Err::<(), _>(format!("Expected Response")));
                    unreachable!()
                }
            }
        });
        handles.push(handle);
    }

    // Collect all sequence numbers
    let mut seq_numbers: Vec<i64> = handles
        .into_iter()
        .map(|h| match h.join() {
            Ok(seq) => seq,
            Err(_) => {
                must(Err::<(), _>("Thread joined with error"));
                unreachable!()
            }
        })
        .collect();

    // Verify all unique
    seq_numbers.sort_unstable();
    seq_numbers.dedup();
    assert_eq!(seq_numbers.len(), 10, "All sequence numbers should be unique");
}

#[test]
// AC:5.3
fn test_thread_safe_session_state() {
    // Test that session state is properly synchronized across threads
    let (tx, _rx) = channel();
    let mut adapter = DebugAdapter::new();
    adapter.set_event_sender(tx.clone());
    let adapter = Arc::new(Mutex::new(adapter));

    let mut handles = vec![];

    // Thread 1: Initialize
    let adapter1 = Arc::clone(&adapter);
    let h1 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        let mut adapter = must(adapter1.lock());
        adapter.handle_request(1, "initialize", None)
    });
    handles.push(h1);

    // Thread 2: Set breakpoints
    let adapter2 = Arc::clone(&adapter);
    let h2 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        let mut adapter = must(adapter2.lock());
        let args = json!({
            "source": { "path": "/test.pl" },
            "breakpoints": [{ "line": 10 }]
        });
        adapter.handle_request(2, "setBreakpoints", Some(args))
    });
    handles.push(h2);

    // Thread 3: Disconnect
    let adapter3 = Arc::clone(&adapter);
    let h3 = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        let mut adapter = must(adapter3.lock());
        adapter.handle_request(3, "disconnect", None)
    });
    handles.push(h3);

    // All threads should complete successfully
    for handle in handles {
        let response = must(handle.join());
        match response {
            DapMessage::Response { success, .. } => {
                assert!(success, "Request should succeed");
            }
            _ => must(Err::<(), _>(format!("Expected Response"))),
        }
    }
}

#[test]
// AC:5.3
fn test_thread_safe_breakpoint_storage() {
    // Test that breakpoint storage is thread-safe
    let adapter = Arc::new(Mutex::new(DebugAdapter::new()));
    let mut handles = vec![];

    // Spawn threads setting breakpoints in different files
    for i in 0..5 {
        let adapter_clone = Arc::clone(&adapter);
        let handle = thread::spawn(move || {
            let mut adapter = must(adapter_clone.lock());
            let args = json!({
                "source": { "path": format!("/test{}.pl", i) },
                "breakpoints": [{ "line": 10 + i }]
            });
            adapter.handle_request(i, "setBreakpoints", Some(args))
        });
        handles.push(handle);
    }

    // All threads should complete successfully
    for handle in handles {
        let response = must(handle.join());
        match response {
            DapMessage::Response { success, .. } => {
                assert!(success, "setBreakpoints should succeed");
            }
            _ => must(Err::<(), _>(format!("Expected Response"))),
        }
    }
}

// ============================================================================
// AC5.4: Error Handling Tests
// ============================================================================

#[test]
// AC:5.4
fn test_error_handling_invalid_command() {
    // Test that invalid commands return proper error responses
    let (mut adapter, _rx) = create_test_adapter();

    let response = adapter.handle_request(1, "invalidCommand", None);

    match response {
        DapMessage::Response { success, command, message, body, .. } => {
            assert!(!success, "Invalid command should fail");
            assert_eq!(command, "invalidCommand");
            assert!(message.is_some());
            assert!(must_some(message).contains("Unknown command"));
            assert!(body.is_none());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.4
fn test_error_handling_malformed_arguments() {
    // Test that malformed arguments are handled gracefully
    let (mut adapter, _rx) = create_test_adapter();

    // setBreakpoints with missing source
    let args = json!({
        "breakpoints": [{ "line": 10 }]
        // Missing "source" field
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Invalid arguments should fail");
            assert!(message.is_some());
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.4
fn test_error_handling_evaluate_with_newlines() {
    // Test that evaluate rejects expressions with newlines (security)
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "expression": "print 'hello';\nsystem('rm -rf /')",
        "frameId": 1
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Expression with newlines should be rejected");
            assert!(message.is_some());
            let msg = must_some(message);
            assert!(msg.contains("newline"), "Error should mention newlines: {}", msg);
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.4
fn test_error_handling_evaluate_empty_expression() {
    // Test that evaluate rejects empty expressions
    let (mut adapter, _rx) = create_test_adapter();

    let args = json!({
        "expression": "",
        "frameId": 1
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Empty expression should be rejected");
            assert!(message.is_some());
            assert!(must_some(message).contains("Empty"));
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.4
fn test_error_handling_scopes_missing_frame_id() {
    // Test that scopes request validates frameId
    let (mut adapter, _rx) = create_test_adapter();

    // Missing frameId argument
    let response = adapter.handle_request(1, "scopes", None);

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Scopes without frameId should fail");
            assert!(message.is_some());
            assert!(must_some(message).contains("frameId"));
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.4
fn test_error_handling_variables_missing_reference() {
    // Test that variables request validates variablesReference
    let (mut adapter, _rx) = create_test_adapter();

    // Missing variablesReference argument
    let response = adapter.handle_request(1, "variables", None);

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Variables without reference should fail");
            assert!(message.is_some());
            assert!(must_some(message).contains("Missing arguments"));
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

#[test]
// AC:5.4
fn test_error_handling_launch_program_is_directory() {
    // Test that launch rejects directory paths
    let (mut adapter, _rx) = create_test_adapter();

    // Use current directory as program (should fail)
    let args = json!({
        "program": ".",
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch with directory should fail");
            assert!(message.is_some());
            let msg = must_some(message);
            assert!(
                msg.contains("not a regular file") || msg.contains("directory"),
                "Error should mention file type: {}",
                msg
            );
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
// AC:5.5
fn test_complete_session_lifecycle() {
    // Test complete session lifecycle from start to finish
    let (mut adapter, rx) = create_test_adapter();

    // 1. Initialize
    let init_resp = adapter.handle_request(1, "initialize", None);
    match init_resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }
    let init_event = wait_for_event(&rx, 100);
    assert!(matches!(init_event, Some(DapMessage::Event { .. })));

    // 2. Set breakpoints
    let bp_resp = adapter.handle_request(
        2,
        "setBreakpoints",
        Some(json!({
            "source": { "path": "/test.pl" },
            "breakpoints": [{ "line": 10 }]
        })),
    );
    match bp_resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }

    // 3. Configuration done
    let config_resp = adapter.handle_request(3, "configurationDone", None);
    match config_resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }

    // 4. Query threads
    let threads_resp = adapter.handle_request(4, "threads", None);
    match threads_resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }

    // 5. Disconnect
    let disconnect_resp = adapter.handle_request(5, "disconnect", None);
    match disconnect_resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => must(Err::<(), _>(format!("Expected Response"))),
    }

    // Should emit terminated event
    let term_event = wait_for_event(&rx, 100);
    assert!(matches!(term_event, Some(DapMessage::Event { .. })));
}

#[test]
// AC:5.5
fn test_multiple_sessions_sequential() {
    // Test that multiple debug sessions can be run sequentially
    for _iteration in 0..3 {
        let (mut adapter, rx) = create_test_adapter();

        // Initialize
        let init_resp = adapter.handle_request(1, "initialize", None);
        match init_resp {
            DapMessage::Response { success, .. } => assert!(success),
            _ => must(Err::<(), _>(format!("Expected Response"))),
        }
        let _init_event = wait_for_event(&rx, 100);

        // Disconnect
        let disconnect_resp = adapter.handle_request(2, "disconnect", None);
        match disconnect_resp {
            DapMessage::Response { success, .. } => assert!(success),
            _ => must(Err::<(), _>(format!("Expected Response"))),
        }
        let _term_event = wait_for_event(&rx, 100);
    }
}
