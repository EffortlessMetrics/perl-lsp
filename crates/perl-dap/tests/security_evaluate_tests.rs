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

/// Comprehensive test for all unsafe operations that must be blocked in safe evaluation mode.
/// These operations can cause:
/// - Code execution (eval, require, do)
/// - Process control issues (kill, exit, dump, fork, alarm, sleep, wait, waitpid)
/// - I/O side effects (print, say, printf, sysread, syswrite)
/// - Filesystem modification (chroot, truncate, symlink, link)
/// - Network operations (socket, connect, bind, listen, accept, send, recv)
/// - Arbitrary code via tie mechanism (tie, untie)
#[test]
fn test_evaluate_blocks_dangerous_operations() {
    let mut adapter = DebugAdapter::new();

    // Map of operation -> example expression that uses it
    // Coverage: all categories of dangerous operations
    let dangerous_ops = [
        // Code execution
        ("eval", "eval('1+1')"),
        ("require", "require 'File.pm'"),
        ("do", "do 'script.pl'"),
        // Process control
        ("kill", "kill 9, $$"),
        ("exit", "exit(0)"),
        ("dump", "dump"),
        ("fork", "fork"),
        ("alarm", "alarm(60)"),
        ("sleep", "sleep(1)"),
        ("wait", "wait"),
        ("waitpid", "waitpid(-1, 0)"),
        // I/O
        ("print", "print 'side effect'"),
        ("say", "say 'side effect'"),
        ("printf", "printf '%s', 'effect'"),
        ("sysread", "sysread(FH, $buf, 100)"),
        ("syswrite", "syswrite(FH, 'data')"),
        // Filesystem
        ("chroot", "chroot('/tmp')"),
        ("truncate", "truncate('file', 0)"),
        ("symlink", "symlink('old', 'new')"),
        ("link", "link('old', 'new')"),
        // Tie mechanism
        ("tie", "tie %hash, 'Tie::Hash'"),
        ("untie", "untie %hash"),
        // Network
        ("socket", "socket(S, PF_INET, SOCK_STREAM, 0)"),
        ("connect", "connect(S, $addr)"),
        ("bind", "bind(S, $addr)"),
        ("listen", "listen(S, 5)"),
        ("accept", "accept(C, S)"),
        ("send", "send(S, 'data', 0)"),
        ("recv", "recv(S, $buf, 100, 0)"),
    ];

    let mut failures = Vec::new();

    for (op_name, expression) in dangerous_ops {
        let args = json!({
            "expression": expression,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        match response {
            DapMessage::Response { success, message, .. } => {
                let msg = message.unwrap_or_default();
                let expected_pattern = format!("potentially mutating operation '{}'", op_name);

                if !success && msg.contains(&expected_pattern) {
                    // Blocked correctly
                } else {
                    failures.push(format!(
                        "Operation '{}' (expr: '{}') was NOT blocked. success={}, msg={}",
                        op_name, expression, success, msg
                    ));
                }
            }
            _ => failures.push(format!("Operation '{}': expected Response, got Event", op_name)),
        }
    }

    if !failures.is_empty() {
        panic!(
            "The following dangerous operations were NOT blocked in safe mode:\n{}",
            failures.join("\n")
        );
    }
}

/// Test that dangerous operations ARE allowed when allowSideEffects is true
#[test]
fn test_evaluate_allows_dangerous_ops_with_side_effects_enabled() {
    let mut adapter = DebugAdapter::new();

    // These should NOT be blocked when allowSideEffects is true
    // (they may still fail for other reasons like not being in a debug session)
    let ops_to_test = ["eval('1')", "print 'test'"];

    for expression in ops_to_test {
        let args = json!({
            "expression": expression,
            "allowSideEffects": true
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        if let DapMessage::Response { message, .. } = response {
            let msg = message.unwrap_or_default();
            // Should NOT be blocked by safe mode validation
            assert!(
                !msg.contains("Safe evaluation mode"),
                "Operation '{}' should NOT be blocked when allowSideEffects=true, but got: {}",
                expression,
                msg
            );
        }
        // Events are fine, just checking we don't get safe-mode rejection
    }
}
