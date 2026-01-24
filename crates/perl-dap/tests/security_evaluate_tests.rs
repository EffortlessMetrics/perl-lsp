#![allow(clippy::unwrap_used, clippy::expect_used)]

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_evaluate_rejects_newlines() {
    let mut adapter = DebugAdapter::new();

    // Malicious expression with newline
    let args = json!({
        "expression": "1\nprint 'hacked'"
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for expression with newlines");
            let msg = message.expect("Should have error message");
            assert_eq!(
                msg, "Expression cannot contain newlines",
                "Should reject newlines explicitly"
            );
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn test_evaluate_detects_unsafe_backticks() {
    let mut adapter = DebugAdapter::new();

    // Expression with backticks (shell execution)
    let args = json!({
        "expression": "`ls -la`",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for backticks in safe mode");
            let msg = message.expect("Should have error message");
            assert!(
                msg.contains("Safe evaluation mode: backticks"),
                "Should specifically mention backticks"
            );
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn test_evaluate_detects_unsafe_qx() {
    let mut adapter = DebugAdapter::new();

    // Expression with qx (shell execution)
    let args = json!({
        "expression": "qx(ls -la)",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for qx in safe mode");
            let msg = message.expect("Should have error message");
            assert!(
                msg.contains("Safe evaluation mode: potentially mutating operation 'qx'"),
                "Should specifically mention qx"
            );
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn test_evaluate_rejects_carriage_returns() {
    let mut adapter = DebugAdapter::new();

    // Malicious expression with carriage return
    let args = json!({
        "expression": "1\rprint 'hacked'"
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for expression with carriage returns");
            let msg = message.expect("Should have error message");
            assert_eq!(
                msg, "Expression cannot contain newlines",
                "Should reject newlines explicitly"
            );
        }
        _ => panic!("Expected Response"),
    }
}
