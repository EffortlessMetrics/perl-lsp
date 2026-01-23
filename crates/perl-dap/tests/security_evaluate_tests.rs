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
fn test_evaluate_rejects_qx() {
    let mut adapter = DebugAdapter::new();

    // Malicious expression using qx
    let args = json!({
        "expression": "qx/ls/",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for expression with qx");
            let msg = message.expect("Should have error message");
            assert!(
                msg.contains("Safe evaluation mode"),
                "Should reject qx in safe mode. Got: {}",
                msg
            );
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn test_evaluate_rejects_backticks() {
    let mut adapter = DebugAdapter::new();

    // Malicious expression using backticks
    let args = json!({
        "expression": "`ls`",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for expression with backticks");
            let msg = message.expect("Should have error message");
            assert!(
                msg.contains("Safe evaluation mode"),
                "Should reject backticks in safe mode. Got: {}",
                msg
            );
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn test_evaluate_rejects_syscall() {
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "expression": "syscall(1)",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for expression with syscall");
            let msg = message.expect("Should have error message");
            assert!(
                msg.contains("Safe evaluation mode"),
                "Should reject syscall in safe mode. Got: {}",
                msg
            );
        }
        _ => panic!("Expected Response"),
    }
}

#[test]
fn test_evaluate_rejects_eval() {
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "expression": "eval 'system(\"ls\")'",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Evaluate should fail for expression with eval");
            let msg = message.expect("Should have error message");
            assert!(
                msg.contains("Safe evaluation mode"),
                "Should reject eval in safe mode. Got: {}",
                msg
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
