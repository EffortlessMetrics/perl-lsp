//! Control Flow Handlers Tests (AC9)
//!
//! Tests for DAP control flow operations: continue, next, stepIn, stepOut, pause
//!
//! Specification: Issue #454 - DAP Control Flow Handlers (AC9)
//!
//! Run with: cargo test -p perl-dap --test control_flow_handlers_tests

use perl_dap::{DapMessage, DebugAdapter};
use perl_tdd_support::{must, must_some};
use serde_json::json;

// AC9.1: Test continue request handler
#[test]
fn test_continue_handler() {
    // AC9: continue without an active session must return success: false with guidance.
    // (#898: fake success was a bug — execution-control handlers now return protocol-safe errors.)
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "continue", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "continue without session must fail");
            assert_eq!(command, "continue");
            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "error must indicate no session: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response message for continue"));
            unreachable!()
        }
    }
}

// AC9.1: Test next (step over) request handler
#[test]
fn test_next_handler() {
    // AC9: next without an active session must return success: false with guidance.
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "next", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "next without session must fail");
            assert_eq!(command, "next");
            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "error must indicate no session: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response message for next"));
            unreachable!()
        }
    }
}

// AC9.1: Test stepIn request handler
#[test]
fn test_step_in_handler() {
    // AC9: stepIn without an active session must return success: false with guidance.
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stepIn", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "stepIn without session must fail");
            assert_eq!(command, "stepIn");
            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "error must indicate no session: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response message for stepIn"));
            unreachable!()
        }
    }
}

// AC9.1: Test stepOut request handler
#[test]
fn test_step_out_handler() {
    // AC9: stepOut without an active session must return success: false with guidance.
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stepOut", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "stepOut without session must fail");
            assert_eq!(command, "stepOut");
            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "error must indicate no session: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response message for stepOut"));
            unreachable!()
        }
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

            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "pause error must indicate no session: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response message for pause"));
            unreachable!()
        }
    }
}

// AC9.4: Test control flow state transitions
#[test]
fn test_control_flow_state_transitions() {
    // AC9 / #898: All four execution-control handlers must return success: false
    // with a guidance message when no debug session is active.
    let mut adapter = DebugAdapter::new();

    let continue_response = adapter.handle_request(1, "continue", None);
    assert!(matches!(continue_response, DapMessage::Response { success: false, .. }));

    let next_response = adapter.handle_request(2, "next", None);
    assert!(matches!(next_response, DapMessage::Response { success: false, .. }));

    let step_in_response = adapter.handle_request(3, "stepIn", None);
    assert!(matches!(step_in_response, DapMessage::Response { success: false, .. }));

    let step_out_response = adapter.handle_request(4, "stepOut", None);
    assert!(matches!(step_out_response, DapMessage::Response { success: false, .. }));
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
        _ => {
            must(Err::<(), _>("Expected Response message"));
            unreachable!()
        }
    }
}

// AC9.1: Test continue with threadId argument
#[test]
fn test_continue_with_thread_id() {
    // AC9 / #898: continue with threadId but no session must still fail gracefully.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "continue", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "continue without session must fail even with threadId");
            assert_eq!(command, "continue");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response message for continue with threadId"));
            unreachable!()
        }
    }
}

// AC9.1: Test next with threadId argument
#[test]
fn test_next_with_thread_id() {
    // AC9 / #898: next with threadId but no session must fail gracefully.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "next", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "next without session must fail even with threadId");
            assert_eq!(command, "next");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response message for next with threadId"));
            unreachable!()
        }
    }
}

// AC9.1: Test stepIn with optional targetId
#[test]
fn test_step_in_with_target_id() {
    // AC9 / #898: stepIn with targetId but no session must fail gracefully.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "targetId": 5
    });

    let response = adapter.handle_request(1, "stepIn", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "stepIn without session must fail even with targetId");
            assert_eq!(command, "stepIn");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response message for stepIn with targetId"));
            unreachable!()
        }
    }
}

// AC9.1: Test stepOut with threadId argument
#[test]
fn test_step_out_with_thread_id() {
    // AC9 / #898: stepOut with threadId but no session must fail gracefully.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1
    });

    let response = adapter.handle_request(1, "stepOut", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "stepOut without session must fail even with threadId");
            assert_eq!(command, "stepOut");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response message for stepOut with threadId"));
            unreachable!()
        }
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
        _ => {
            must(Err::<(), _>("Expected Response message for pause with threadId"));
            unreachable!()
        }
    }
}

// AC9.4: Test multiple sequential control flow operations
#[test]
fn test_sequential_control_flow_operations() {
    // AC9 / #898: All four handlers return failure without a session.
    // Each response must have the correct command name and success: false.
    let mut adapter = DebugAdapter::new();

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
            DapMessage::Response { success, command: resp_cmd, message, .. } => {
                assert!(!success, "Operation {} must fail without session", command);
                assert_eq!(&resp_cmd, command, "Command should match");
                assert!(message.is_some(), "{command} must include guidance message");
            }
            _ => must(Err::<(), _>(format!("Expected Response for command {}", command))),
        }
    }
}

// AC9.5: Test edge case - continue with missing threadId
#[test]
fn test_continue_missing_thread_id() {
    // AC9 / #898: continue without session must fail even with empty args object.
    let mut adapter = DebugAdapter::new();

    let args = json!({});

    let response = adapter.handle_request(1, "continue", Some(args));

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "continue without session must fail (empty args)");
            assert_eq!(command, "continue");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response message"));
            unreachable!()
        }
    }
}

// AC9.5: Test edge case - operations with null arguments
#[test]
fn test_control_flow_with_null_arguments() {
    // AC9 / #898: All four execution-control handlers fail without a session,
    // even when called with null arguments.
    let mut adapter = DebugAdapter::new();

    let commands = vec!["continue", "next", "stepIn", "stepOut"];

    for command in commands {
        let response = adapter.handle_request(1, command, None);

        match response {
            DapMessage::Response { success, message, .. } => {
                assert!(!success, "{command} must fail without session (null args)");
                assert!(
                    message.is_some(),
                    "{command} must include guidance message with null args"
                );
            }
            _ => must(Err::<(), _>(format!("Expected Response for {}", command))),
        }
    }
}

// AC9.4: Test response format consistency
#[test]
fn test_control_flow_response_format() {
    // AC9 / #898: Response format is consistent for failure (no-session) path too:
    // positive seq, matching request_seq, success: false, correct command name.
    let mut adapter = DebugAdapter::new();

    let commands = vec!["continue", "next", "stepIn", "stepOut"];

    for command in commands {
        let response = adapter.handle_request(1, command, None);

        match response {
            DapMessage::Response { seq, request_seq, success, command: cmd, message, .. } => {
                assert!(seq > 0, "Sequence number should be positive");
                assert_eq!(request_seq, 1, "Request sequence should match");
                assert!(!success, "{command} must fail without session");
                assert_eq!(cmd, command, "Command name should match");
                assert!(message.is_some(), "{command} failure must include guidance message");
            }
            _ => must(Err::<(), _>(format!("Expected Response for {}", command))),
        }
    }
}

// AC9.1: Verify Perl debugger command mapping
#[test]
fn test_perl_debugger_command_mapping() {
    // AC9 / #898: Without a session, all four handlers fail gracefully.
    // The Perl debugger command mapping (c/n/s/r) is exercised only with a live session.
    let mut adapter = DebugAdapter::new();

    assert!(matches!(
        adapter.handle_request(1, "continue", None),
        DapMessage::Response { success: false, .. }
    ));

    assert!(matches!(
        adapter.handle_request(2, "next", None),
        DapMessage::Response { success: false, .. }
    ));

    assert!(matches!(
        adapter.handle_request(3, "stepIn", None),
        DapMessage::Response { success: false, .. }
    ));

    assert!(matches!(
        adapter.handle_request(4, "stepOut", None),
        DapMessage::Response { success: false, .. }
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
        _ => {
            must(Err::<(), _>("Expected Response for pause"));
            unreachable!()
        }
    }
}

// AC9.4: Test continue response includes allThreadsContinued
#[test]
fn test_continue_includes_all_threads_continued() {
    // AC9 / #898: Without a session, continue returns success: false and body: None.
    // The allThreadsContinued body field is only present on the success path.
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "continue", None);

    if let DapMessage::Response { success, body, message, .. } = response {
        assert!(!success, "continue without session must fail");
        assert!(body.is_none(), "failure response must have no body");
        assert!(message.is_some(), "failure must include guidance message");
    } else {
        must(Err::<(), _>("Expected Response for continue"));
        unreachable!();
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
        _ => {
            must(Err::<(), _>("Expected Response for unknown command"));
            unreachable!()
        }
    }
}

#[test]
fn test_unknown_command_includes_case_only_suggestion() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "STEPIN", None);

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Unexpected success for unknown command");
            let msg = must_some(message);
            assert!(
                msg.contains("Unknown command: STEPIN"),
                "Error should include unknown command: {msg}"
            );
            assert!(
                msg.contains("Did you mean 'stepIn'?"),
                "Error should include case-only suggestion: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response for unknown command"));
            unreachable!()
        }
    }
}

#[test]
fn test_unknown_command_includes_typo_suggestion() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "setBreakpoint", None);

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Unexpected success for unknown command");
            let msg = must_some(message);
            assert!(
                msg.contains("Unknown command: setBreakpoint"),
                "Error should include unknown command: {msg}"
            );
            assert!(
                msg.contains("Did you mean 'setBreakpoints'?"),
                "Error should include typo suggestion: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected Response for unknown command"));
            unreachable!()
        }
    }
}

// AC9.4: Test that handlers are thread-safe (can be called multiple times)
#[test]
fn test_control_flow_handlers_thread_safe() {
    // AC9 / #898: Handlers must be reusable; each call without a session returns
    // a consistent failure response (not a panic or a different shape).
    let mut adapter = DebugAdapter::new();

    for i in 1..=5 {
        let response = adapter.handle_request(i, "next", None);

        match response {
            DapMessage::Response { success, message, command, .. } => {
                assert!(!success, "next without session must fail on iteration {i}");
                assert_eq!(command, "next");
                assert!(message.is_some(), "must include guidance message on iteration {i}");
            }
            _ => must(Err::<(), _>(format!("Expected Response on iteration {}", i))),
        }
    }
}

// AC9.1: Test stepIn with granularity argument (future enhancement)
#[test]
fn test_step_in_with_granularity() {
    // AC9 / #898: stepIn fails without a session regardless of granularity argument.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "granularity": "statement"
    });

    let response = adapter.handle_request(1, "stepIn", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "stepIn without session must fail even with granularity");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response for stepIn with granularity"));
            unreachable!()
        }
    }
}

// AC9.1: Test next with granularity argument (future enhancement)
#[test]
fn test_next_with_granularity() {
    // AC9 / #898: next fails without a session regardless of granularity argument.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "granularity": "line"
    });

    let response = adapter.handle_request(1, "next", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "next without session must fail even with granularity");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response for next with granularity"));
            unreachable!()
        }
    }
}

// AC9.1: Test stepOut with granularity argument (future enhancement)
#[test]
fn test_step_out_with_granularity() {
    // AC9 / #898: stepOut fails without a session regardless of granularity argument.
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "threadId": 1,
        "granularity": "statement"
    });

    let response = adapter.handle_request(1, "stepOut", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "stepOut without session must fail even with granularity");
            assert!(message.is_some(), "must include guidance message");
        }
        _ => {
            must(Err::<(), _>("Expected Response for stepOut with granularity"));
            unreachable!()
        }
    }
}

// ── Proving tests for #898 ────────────────────────────────────────────────────

/// Proving test: continue without a session returns a guidance message.
#[test]
// AC:898
fn continue_without_session_returns_guidance() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "continue", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "continue");
            assert!(!success, "continue without a session must fail");
            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "guidance must mention no session: {msg}"
            );
            assert!(
                msg.contains("Start a launch or attach request"),
                "guidance must give actionable advice: {msg}"
            );
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}

/// Proving test: next without a session returns a guidance message.
#[test]
// AC:898
fn next_without_session_returns_guidance() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "next", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "next");
            assert!(!success, "next without a session must fail");
            let msg = must_some(message);
            assert!(msg.contains("no Perl debug session is active"), "got: {msg}");
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}

/// Proving test: stepIn without a session returns a guidance message.
#[test]
// AC:898
fn step_in_without_session_returns_guidance() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stepIn", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "stepIn");
            assert!(!success, "stepIn without a session must fail");
            let msg = must_some(message);
            assert!(msg.contains("no Perl debug session is active"), "got: {msg}");
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}

/// Proving test: stepOut without a session returns a guidance message.
#[test]
// AC:898
fn step_out_without_session_returns_guidance() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "stepOut", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "stepOut");
            assert!(!success, "stepOut without a session must fail");
            let msg = must_some(message);
            assert!(msg.contains("no Perl debug session is active"), "got: {msg}");
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}
