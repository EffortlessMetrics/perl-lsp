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
fn test_evaluate_safe_mode_blocks_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "$x = 42",
        "allowSideEffects": false
    });
    let response = adapter.handle_request(1, "evaluate", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        assert!(!success);
        assert!(message.ok_or("Expected error message")?.contains("assignment operator"));
    }
    Ok(())
}

#[test]
// Sentinel Security Fix: Block iterator state mutation and file handle reads
fn test_evaluate_safe_mode_blocks_iterator_and_io() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let unsafe_ops = vec![
        "each %hash",
        "keys %hash",
        "values %hash",
        "<$fh>",
        "<STDIN>",
        "<ARGV>",
        "eof",
        "eof $fh",
        "1 + <*.*>", // Glob
    ];

    for op in unsafe_ops {
        let args = json!({
            "expression": op,
            "allowSideEffects": false,
            "context": "hover"
        });
        let response = adapter.handle_request(1, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            assert!(!success, "Operation '{}' should have failed", op);
            if let Some(msg) = message {
                 assert!(msg.contains("Safe evaluation mode"), "Operation '{}' failed but not due to safety: {}", op, msg);
            } else {
                 panic!("Operation '{}' failed without message", op);
            }
        }
    }
    Ok(())
}

#[test]
// Sentinel Security Fix: Ensure safe operations are still allowed
fn test_evaluate_safe_mode_allows_comparisons_and_lookups() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let safe_ops = vec![
        "$a < $b",
        "$a > $b",
        "$hash{key}",
        "$keys",
        "$values",
        "$each",
    ];

    for op in safe_ops {
        let args = json!({
            "expression": op,
            "allowSideEffects": false,
            "context": "hover"
        });
        let response = adapter.handle_request(1, "evaluate", Some(args));

        // It should NOT fail with "Safe evaluation mode"
        // It likely fails with "No debugger session" or succeeds if mocked
        if let DapMessage::Response { message, .. } = response {
            if let Some(msg) = message {
                assert!(!msg.contains("Safe evaluation mode"), "Safe operation '{}' was blocked: {}", op, msg);
            }
        }
    }
    Ok(())
}

#[test]
// AC:10.2
fn test_evaluate_safe_mode_blocks_mutation() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "push @arr, 1",
        "allowSideEffects": false
    });
    let response = adapter.handle_request(1, "evaluate", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        assert!(!success);
        assert!(
            message.ok_or("Expected error message")?.contains("potentially mutating operation")
        );
    }
    Ok(())
}

#[test]
// AC:10.2
fn test_evaluate_allows_side_effects_opt_in() -> Result<(), Box<dyn std::error::Error>> {
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
            assert!(!message.ok_or("Expected error message")?.contains("assignment operator"));
        }
    }
    Ok(())
}

#[test]
// AC:10.3
fn test_evaluate_timeout_enforcement_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();
    let args = json!({
        "expression": "1 + 1",
        "timeout": 60000 // Above hard limit
    });
    let response = adapter.handle_request(1, "evaluate", Some(args));

    match response {
        DapMessage::Response { body, success, .. } => {
            if success {
                let body_val = body.ok_or("Expected body in response")?;
                let result = body_val
                    .get("result")
                    .ok_or("Expected result field")?
                    .as_str()
                    .ok_or("Expected string result")?;
                // Should cap timeout at 30000ms
                assert!(result.contains("30000ms"));
            }
        }
        _ => return Err("Expected response".into()),
    }
    Ok(())
}
