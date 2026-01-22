//! Security regression tests for DAP debug adapter
//!
//! These tests verify that command injection vulnerabilities are properly mitigated
//! in the debug adapter's program launch functionality.

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

/// Test that `-e` flag injection is blocked
///
/// Vulnerability: If program argument accepts "-e", Perl would interpret the
/// "args" as code to execute rather than script arguments.
#[test]
fn test_command_injection_via_program_argument() {
    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": "-e",
        "args": ["BEGIN { print \"pwned\\n\"; } exit;"]
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    // Verify response indicates failure (due to file "-e" not found)
    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail because file '-e' does not exist");
            assert!(
                message.unwrap().contains("does not exist"),
                "Should fail with file not found error"
            );
        }
        _ => panic!("Expected Response"),
    }

    // Give it a moment to potentially run and produce output (if vulnerable)
    thread::sleep(Duration::from_millis(500));

    // Check if we received any output containing "pwned"
    let mut found_pwned = false;
    while let Ok(msg) = rx.try_recv() {
        if let DapMessage::Event { event, body: Some(body), .. } = msg
            && event == "output"
            && let Some(output) = body.get("output").and_then(|o| o.as_str())
            && output.contains("pwned")
        {
            found_pwned = true;
        }
    }

    // Assert that we don't see "pwned" (this should pass after fix)
    assert!(!found_pwned, "Should not execute arbitrary code via -e");
}

/// Test that non-existent files are rejected gracefully
#[test]
fn test_launch_with_nonexistent_file_errors_gracefully() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": "nonexistent_file_12345.pl",
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail for nonexistent file");
            assert!(message.unwrap().contains("does not exist"), "Should return meaningful error");
        }
        _ => panic!("Expected Response"),
    }
}

/// Test that empty program path is rejected
#[test]
fn test_launch_with_empty_program_rejected() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": "",
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail for empty program");
            let msg = message.unwrap();
            assert!(
                msg.contains("empty") || msg.contains("does not exist"),
                "Should indicate empty or invalid path: {}",
                msg
            );
        }
        _ => panic!("Expected Response"),
    }
}

/// Test that whitespace-only program path is rejected
#[test]
fn test_launch_with_whitespace_program_rejected() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": "   \t\n  ",
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail for whitespace-only program");
            let msg = message.unwrap();
            assert!(
                msg.contains("empty") || msg.contains("does not exist"),
                "Should indicate empty or invalid path: {}",
                msg
            );
        }
        _ => panic!("Expected Response"),
    }
}

/// Test that directory paths are rejected (not regular files)
#[test]
fn test_launch_with_directory_rejected() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    // Use a directory that definitely exists
    let args = json!({
        "program": "/tmp",
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail for directory path");
            let msg = message.unwrap();
            assert!(
                msg.contains("not a regular file") || msg.contains("does not exist"),
                "Should indicate path is not a file: {}",
                msg
            );
        }
        _ => panic!("Expected Response"),
    }
}

/// Test that other Perl flags are also blocked
#[test]
fn test_other_flag_injection_blocked() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    // Test various dangerous flags
    for flag in &["-E", "-n", "-p", "-i", "-0", "--"] {
        let args = json!({
            "program": flag,
            "args": []
        });

        let response = adapter.handle_request(1, "launch", Some(args));

        match response {
            DapMessage::Response { success, message, .. } => {
                assert!(!success, "Launch should fail for flag '{}' as program", flag);
                assert!(message.is_some(), "Should have error message for flag '{}'", flag);
            }
            _ => panic!("Expected Response for flag '{}'", flag),
        }
    }
}
