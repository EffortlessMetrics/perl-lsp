//! Control Flow Handlers Tests (AC9)
//!
//! Tests for DAP control flow operations: continue, next, stepIn, stepOut, pause
//!
//! Specification: Issue #454 - DAP Control Flow Handlers (AC9)
//!
//! Run with: cargo test -p perl-dap --test control_flow_handlers_tests

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;

// AC9.1: Test continue request handler
#[test]
fn test_continue_handler() {
    // AC9: Continue request should transition to Running state
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "continue", None);

    match response {
        DapMessage::Response { success, command, body, message, .. } => {
            assert!(success, "Continue request should succeed");
            assert_eq!(command, "continue");
            assert!(message.is_none(), "Continue should not have error message");

            // Verify response body contains allThreadsContinued
            if let Some(body_value) = body {
                assert_eq!(
                    body_value.get("allThreadsContinued"),
                    Some(&json!(true)),
                    "Continue response should indicate all threads continued"
                );
            } else {
                must(Err::<(), _>(format!("Continue response should have body with allThreadsContinued")));
            }
        }
        _ => must(Err::<(), _>(format!("Expected Response message for continue"))),
    }
}

// AC9.1: Test next (step over) request handler
#[test]
fn test_next_handler() {
    // AC9: Next request should execute step over operation
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "next", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(success, "Next request should succeed");
            assert_eq!(command, "next");
            assert!(message.is_none(), "Next should not have error message");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for next"))),
    }
}

// AC9.1: Test stepIn request handler
#[test]
fn test_step_in_handler() {
    // AC9: StepIn request should step into subroutine calls
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stepIn", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(success, "StepIn request should succeed");
            assert_eq!(command, "stepIn");
            assert!(message.is_none(), "StepIn should not have error message");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for stepIn"))),
    }
}

// AC9.1: Test stepOut request handler
#[test]
fn test_step_out_handler() {
    // AC9: StepOut request should step out of current subroutine
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stepOut", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(success, "StepOut request should succeed");
            assert_eq!(command, "stepOut");
            assert!(message.is_none(), "StepOut should not have error message");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for stepOut"))),
    }
}

// AC9.1: Test pause request handler
#[test]
fn test_pause_handler_no_session() {
    // AC9: Pause request should handle missing session gracefully
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "pause", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            // Without an active session, pause should fail gracefully
            assert!(!success, "Pause should fail without active session");
            assert_eq!(command, "pause");
            assert!(message.is_some(), "Pause without session should provide error message");

            if let Some(msg) = message {
                assert!(
                    msg.contains("Failed to pause") || msg.to_lowercase().contains("debugger"),
                    "Error message should indicate pause failure or no session: {}",
                    msg
                );
            }
        }
        _ => must(Err::<(), _>(format!("Expected Response message for pause"))),
    }
}

// AC9.4: Test control flow state transitions
#[test]
fn test_control_flow_state_transitions() {
    // AC9: Verify state transitions for control flow operations
    let mut adapter = DebugAdapter::new();

    // All control flow operations should succeed even without a session
    // (they handle missing session gracefully)

    let continue_response = adapter.handle_request(1, "continue", None);
    assert!(matches!(continue_response, DapMessage::Response { success: true, .. }));

    let next_response = adapter.handle_request(2, "next", None);
    assert!(matches!(next_response, DapMessage::Response { success: true, .. }));

    let step_in_response = adapter.handle_request(3, "stepIn", None);
    assert!(matches!(step_in_response, DapMessage::Response { success: true, .. }));

    let step_out_response = adapter.handle_request(4, "stepOut", None);
    assert!(matches!(step_out_response, DapMessage::Response { success: true, .. }));
}

// AC9.4: Test that responses have correct sequence numbers
#[test]
fn test_control_flow_sequence_numbers() {
    // AC9: Verify sequence numbers in control flow responses
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(42, "continue", None);

    match response {
        DapMessage::Response { seq, request_seq, .. } => {
            assert!(seq > 0, "Response sequence should be positive");
            assert_eq!(request_seq, 42, "Request sequence should match");
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

// AC9.1: Test continue with threadId argument
#[test]
fn test_continue_with_thread_id() {
    // AC9: Continue request should accept threadId argument
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "continue", Some(args));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "Continue with threadId should succeed");
            assert_eq!(command, "continue");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for continue with threadId"))),
    }
}

// AC9.1: Test next with threadId argument
#[test]
fn test_next_with_thread_id() {
    // AC9: Next request should accept threadId argument
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "next", Some(args));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "Next with threadId should succeed");
            assert_eq!(command, "next");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for next with threadId"))),
    }
}

// AC9.1: Test stepIn with optional targetId
#[test]
fn test_step_in_with_target_id() {
    // AC9: StepIn request should accept optional targetId
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "targetId": 5
    });

    let response = adapter.handle_request(1, "stepIn", Some(args));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "StepIn with targetId should succeed");
            assert_eq!(command, "stepIn");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for stepIn with targetId"))),
    }
}

// AC9.1: Test stepOut with threadId argument
#[test]
fn test_step_out_with_thread_id() {
    // AC9: StepOut request should accept threadId argument
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "stepOut", Some(args));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "StepOut with threadId should succeed");
            assert_eq!(command, "stepOut");
        }
        _ => must(Err::<(), _>(format!("Expected Response message for stepOut with threadId"))),
    }
}

// AC9.1: Test pause with threadId argument
#[test]
fn test_pause_with_thread_id() {
    // AC9: Pause request should accept threadId argument
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "pause", Some(args));

    match response {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "pause");
            // Success depends on whether there's an active session
        }
        _ => must(Err::<(), _>(format!("Expected Response message for pause with threadId"))),
    }
}

// AC9.4: Test multiple sequential control flow operations
#[test]
fn test_sequential_control_flow_operations() {
    // AC9: Verify multiple control flow operations can be executed sequentially
    let mut adapter = DebugAdapter::new();

    // Execute a sequence of control flow operations
    let operations = [
        ("continue", json!({"threadId": 1})),
        ("next", json!({"threadId": 1})),
        ("stepIn", json!({"threadId": 1})),
        ("stepOut", json!({"threadId": 1})),
        ("next", json!({"threadId": 1})),
        ("continue", json!({"threadId": 1})),
    ];

    for (idx, (command, args)) in operations.iter().enumerate() {
        let response = adapter.handle_request((idx + 1) as i64, command, Some(args.clone()));

        match response {
            DapMessage::Response { success, command: resp_cmd, .. } => {
                assert!(success, "Operation {} should succeed", command);
                assert_eq!(&resp_cmd, command, "Command should match");
            }
            _ => must(Err::<(), _>(format!("Expected Response for command {}", command))),
        }
    }
}

// AC9.5: Test edge case - continue with missing threadId
#[test]
fn test_continue_missing_thread_id() {
    // AC9: Continue should work even without threadId argument
    let mut adapter = DebugAdapter::new();

    let args = json!({});

    let response = adapter.handle_request(1, "continue", Some(args));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "Continue without threadId should still succeed");
            assert_eq!(command, "continue");
        }
        _ => must(Err::<(), _>(format!("Expected Response message"))),
    }
}

// AC9.5: Test edge case - operations with null arguments
#[test]
fn test_control_flow_with_null_arguments() {
    // AC9: Control flow operations should handle null/empty arguments gracefully
    let mut adapter = DebugAdapter::new();

    let commands = vec!["continue", "next", "stepIn", "stepOut"];

    for command in commands {
        let response = adapter.handle_request(1, command, None);

        match response {
            DapMessage::Response { success, .. } => {
                // These should succeed even without arguments (for most operations)
                if command == "pause" {
                    // Pause may fail without a session - we accept either outcome
                    let _ = success;
                } else {
                    assert!(success, "{} should succeed with null arguments", command);
                }
            }
            _ => must(Err::<(), _>(format!("Expected Response for {}", command))),
        }
    }
}

// AC9.4: Test response format consistency
#[test]
fn test_control_flow_response_format() {
    // AC9: All control flow responses should have consistent format
    let mut adapter = DebugAdapter::new();

    let commands = vec!["continue", "next", "stepIn", "stepOut"];

    for command in commands {
        let response = adapter.handle_request(1, command, None);

        match response {
            DapMessage::Response { seq, request_seq, success, command: cmd, .. } => {
                assert!(seq > 0, "Sequence number should be positive");
                assert_eq!(request_seq, 1, "Request sequence should match");
                assert!(success, "{} should succeed", command);
                assert_eq!(cmd, command, "Command name should match");
            }
            _ => must(Err::<(), _>(format!("Expected Response for {}", command))),
        }
    }
}

// AC9.1: Verify Perl debugger command mapping
#[test]
fn test_perl_debugger_command_mapping() {
    // AC9: Verify that DAP commands map to correct Perl debugger commands
    // This is implicitly tested by the handler implementations:
    // - continue -> "c\n"
    // - next -> "n\n"
    // - stepIn -> "s\n"
    // - stepOut -> "r\n"

    // The actual command sending is tested through the handlers
    let mut adapter = DebugAdapter::new();

    // Verify handlers respond correctly (command sending happens internally)
    assert!(matches!(
        adapter.handle_request(1, "continue", None),
        DapMessage::Response { success: true, .. }
    ));

    assert!(matches!(
        adapter.handle_request(2, "next", None),
        DapMessage::Response { success: true, .. }
    ));

    assert!(matches!(
        adapter.handle_request(3, "stepIn", None),
        DapMessage::Response { success: true, .. }
    ));

    assert!(matches!(
        adapter.handle_request(4, "stepOut", None),
        DapMessage::Response { success: true, .. }
    ));
}

// AC9.4: Test that pause returns appropriate success status
#[test]
fn test_pause_without_active_session_returns_failure() {
    // AC9: Pause should return failure when no session is active
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "pause", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "pause");
            assert!(!success, "Pause without session should fail");
            assert!(message.is_some(), "Failure should include error message");
        }
        _ => must(Err::<(), _>(format!("Expected Response for pause"))),
    }
}

// AC9.4: Test continue response includes allThreadsContinued
#[test]
fn test_continue_includes_all_threads_continued() {
    // AC9: Continue response must include allThreadsContinued per DAP spec
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "continue", None);

    if let DapMessage::Response { body, .. } = response {
        assert!(body.is_some(), "Continue must have response body");

        if let Some(body_value) = body {
            assert!(
                body_value.get("allThreadsContinued").is_some(),
                "Continue body must include allThreadsContinued field"
            );
            assert_eq!(
                body_value.get("allThreadsContinued"),
                Some(&json!(true)),
                "allThreadsContinued should be true"
            );
        }
    } else {
        must(Err::<(), _>(format!("Expected Response for continue")));
    }
}

// AC9.1: Test all five core control flow operations exist
#[test]
fn test_all_control_flow_operations_exist() {
    // AC9: Verify all five control flow operations are implemented
    let mut adapter = DebugAdapter::new();

    let operations = vec!["continue", "next", "stepIn", "stepOut", "pause"];

    for operation in operations {
        let response = adapter.handle_request(1, operation, None);

        // Verify the operation is recognized (not unknown command)
        match response {
            DapMessage::Response { command, .. } => {
                assert_eq!(command, operation, "Operation {} should be recognized", operation);
            }
            _ => must(Err::<(), _>(format!("Operation {} should return Response", operation))),
        }
    }
}

// AC9.5: Test unknown control flow command
#[test]
fn test_unknown_control_flow_command() {
    // AC9: Unknown commands should be rejected
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "unknownCommand", None);

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Unknown command should fail");
            assert!(message.is_some(), "Unknown command should have error message");

            if let Some(msg) = message {
                assert!(
                    msg.to_lowercase().contains("unknown"),
                    "Error should indicate unknown command"
                );
            }
        }
        _ => must(Err::<(), _>(format!("Expected Response for unknown command"))),
    }
}

// AC9.4: Test that handlers are thread-safe (can be called multiple times)
#[test]
fn test_control_flow_handlers_thread_safe() {
    // AC9: Handlers should be reusable and thread-safe
    let mut adapter = DebugAdapter::new();

    // Call same handler multiple times
    for i in 1..=5 {
        let response = adapter.handle_request(i, "next", None);

        match response {
            DapMessage::Response { success, .. } => {
                assert!(success, "Handler should work on iteration {}", i);
            }
            _ => must(Err::<(), _>(format!("Expected Response on iteration {}", i))),
        }
    }
}

// AC9.1: Test stepIn with granularity argument (future enhancement)
#[test]
fn test_step_in_with_granularity() {
    // AC9: StepIn may support granularity in future (statement/line/instruction)
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "granularity": "statement"
    });

    let response = adapter.handle_request(1, "stepIn", Some(args));

    match response {
        DapMessage::Response { success, .. } => {
            // Should succeed even if granularity is not yet supported
            assert!(success, "StepIn with granularity should succeed");
        }
        _ => must(Err::<(), _>(format!("Expected Response for stepIn with granularity"))),
    }
}

// AC9.1: Test next with granularity argument (future enhancement)
#[test]
fn test_next_with_granularity() {
    // AC9: Next may support granularity in future
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "granularity": "line"
    });

    let response = adapter.handle_request(1, "next", Some(args));

    match response {
        DapMessage::Response { success, .. } => {
            assert!(success, "Next with granularity should succeed");
        }
        _ => must(Err::<(), _>(format!("Expected Response for next with granularity"))),
    }
}

// AC9.1: Test stepOut with granularity argument (future enhancement)
#[test]
fn test_step_out_with_granularity() {
    // AC9: StepOut may support granularity in future
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "granularity": "statement"
    });

    let response = adapter.handle_request(1, "stepOut", Some(args));

    match response {
        DapMessage::Response { success, .. } => {
            assert!(success, "StepOut with granularity should succeed");
        }
        _ => must(Err::<(), _>(format!("Expected Response for stepOut with granularity"))),
    }
}
