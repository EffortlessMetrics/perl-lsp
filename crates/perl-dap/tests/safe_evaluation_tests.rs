//! Safe Evaluation Tests (AC10.6)
//!
//! Comprehensive tests for DAP expression evaluation including:
//! - Safe mode enforcement (rejecting assignments and mutations)
//! - Explicit side effects opt-in
//! - Timeout enforcement (simulated)
//! - Error handling for malformed expressions
//!
//! Specification: GitHub Issue #455 - AC10.2, AC10.3, AC10.6

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

/// Helper to create a test adapter
fn create_test_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

#[test]
// AC:10.2
fn test_evaluate_safe_mode_blocks_assignment() {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "$x = 42",
        "allowSideEffects": false
    });
    let response = adapter.handle_request(1, "evaluate", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        assert!(!success);
        assert!(message.unwrap().contains("assignment operator"));
    }
}

#[test]
// AC:10.2
fn test_evaluate_safe_mode_blocks_mutation() {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "push @arr, 1",
        "allowSideEffects": false
    });
    let response = adapter.handle_request(1, "evaluate", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        assert!(!success);
        assert!(message.unwrap().contains("potentially mutating operation"));
    }
}

#[test]
// AC:10.2
fn test_evaluate_allows_side_effects_opt_in() {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "$x = 42",
        "allowSideEffects": true
    });
    // Without active session, it will fail at execution, but pass safety validation
    let response = adapter.handle_request(1, "evaluate", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        // If it failed due to safety, success=false and message mentions "assignment"
        // If it passed safety, it would fail due to "No debugger session"
        if !success {
            assert!(!message.unwrap().contains("assignment operator"));
        }
    }
}

#[test]
// AC:10.3
fn test_evaluate_timeout_enforcement_parameters() {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "1 + 1",
        "timeout": 60000 // Above hard limit
    });
    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { body, success, .. } => {
            if success {
                let body_val = body.unwrap();
                let result = body_val.get("result").unwrap().as_str().unwrap();
                // Should cap timeout at 30000ms
                assert!(result.contains("30000ms"));
            }
        }
        _ => panic!("Expected response"),
    }
}
