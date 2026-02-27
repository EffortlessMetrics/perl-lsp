//! Enhanced edge case tests for DAP debugger output parsing
//!
//! These tests cover various edge cases and complex scenarios for the Perl debugger
//! output parsing to ensure robustness.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;
use std::sync::mpsc::channel;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Create a test Perl script with specific content
fn create_edge_case_script(
    content: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let script_path = dir.path().join("edge_case.pl");
    write(&script_path, content)?;
    Ok(script_path)
}

#[test]
fn test_dap_complex_perl_syntax() -> TestResult {
    // Test with complex Perl syntax that might confuse the debugger parser
    let complex_script = r#"#!/usr/bin/perl
use strict;
use warnings;
use feature 'say';

package MyClass;

sub new {
    my $class = shift;
    my %args = @_;
    return bless { %args }, $class;
}

sub method_with_complex_regex {
    my $self = shift;
    my $text = shift;

    # Complex regex that might confuse parsing
    if ($text =~ m{
        ^
        (?<protocol>https?)://
        (?<domain>[^/]+)
        (?<path>/.*)?
        $
    }x) {
        say "Matched: $+{protocol}, $+{domain}, $+{path}";
    }

    return $self;
}

package main;

my $obj = MyClass->new(name => 'test', value => 42);
$obj->method_with_complex_regex('https://example.com/path');

say "Done";
"#;

    let script_path = create_edge_case_script(complex_script)?;
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    // Test that we can handle complex breakpoint scenarios
    let bp_args = json!({
        "source": {"path": script_path.to_str().ok_or("Failed to convert path to string")?},
        "breakpoints": [
            {"line": 10, "condition": "$args{name} eq 'test'"},
            {"line": 20},
            {"line": 30, "condition": "defined($+{domain})"}
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(bp_args));
    match response {
        DapMessage::Response { success, body, .. } => {
            assert!(success);
            let body = body.ok_or("Expected body in response")?;
            let breakpoints = body
                .get("breakpoints")
                .and_then(|b| b.as_array())
                .ok_or("Expected breakpoints array")?;
            assert_eq!(breakpoints.len(), 3);

            // All breakpoints should be present (verified depends on session)
            for bp in breakpoints {
                assert!(bp.get("line").is_some());
                assert!(bp.get("id").is_some());
            }
        }
        _ => return Err("Expected successful setBreakpoints response".into()),
    }
    Ok(())
}

#[test]
fn test_dap_evaluate_complex_expressions() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test various complex expressions that might be evaluated in debugger
    let test_expressions = vec![
        "$hash{complex_key}",
        "@array[0..5]",
        "scalar(@_)",
        "$obj->method()",
        "exists($hash{key})",
        "defined($maybe_undef)",
        "ref($reference)",
        "$complex_var->{nested}->[0]",
        "join(',', @array)",
        "map { $_ * 2 } @numbers",
    ];

    for expr in test_expressions {
        let eval_args = json!({
            "expression": expr,
            "context": "repl"
        });

        let response = adapter.handle_request(1, "evaluate", Some(eval_args));
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert_eq!(command, "evaluate");
                if !success {
                    // Expected when no session - check error message is reasonable
                    if let Some(msg) = message {
                        assert!(msg.contains("debugger session") || msg.contains("session"));
                    }
                }
            }
            _ => return Err(format!("Expected evaluate response for expression: {}", expr).into()),
        }
    }
    Ok(())
}

#[test]
fn test_dap_variables_complex_scopes() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test different variable reference scenarios
    let test_cases = vec![
        11,  // Local scope (frame 1)
        12,  // Package scope (frame 1)
        0,   // Invalid reference
        -1,  // Invalid negative reference
        999, // Very high reference
    ];

    for var_ref in test_cases {
        let var_args = json!({
            "variablesReference": var_ref
        });

        let response = adapter.handle_request(1, "variables", Some(var_args));
        match response {
            DapMessage::Response { success, command, body, .. } => {
                assert_eq!(command, "variables");
                if success {
                    let body = body.ok_or("Expected body in successful response")?;
                    let variables = body
                        .get("variables")
                        .and_then(|v| v.as_array())
                        .ok_or("Expected variables array")?;
                    // Should return placeholders for valid refs even without session
                    if var_ref == 11 {
                        assert!(
                            !variables.is_empty(),
                            "Local scope should have placeholder variables"
                        );
                    }
                } else {
                    // Invalid references should fail gracefully
                    if var_ref <= 0 || var_ref > 100 {
                        // Expected for invalid references
                    }
                }
            }
            _ => {
                return Err(
                    format!("Expected variables response for reference: {}", var_ref).into()
                );
            }
        }
    }
    Ok(())
}

#[test]
fn test_dap_stack_trace_edge_cases() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test stack trace with various thread IDs and scenarios
    let test_cases = [
        Some(json!({"threadId": 1, "startFrame": 0, "levels": 10})),
        Some(json!({"threadId": 1, "startFrame": 5, "levels": 5})),
        Some(json!({"threadId": 999})), // Invalid thread
        Some(json!({"levels": 0})),     // Zero levels
        None,                           // No arguments
    ];

    for (i, args) in test_cases.iter().enumerate() {
        let response = adapter.handle_request(i as i64 + 1, "stackTrace", args.clone());
        match response {
            DapMessage::Response { success, command, body, .. } => {
                assert_eq!(command, "stackTrace");
                if success {
                    let body = body.ok_or("Expected body in successful response")?;
                    let frames = body
                        .get("stackFrames")
                        .and_then(|f| f.as_array())
                        .ok_or("Expected frames array")?;
                    // Without active session, should return 1 placeholder frame
                    assert_eq!(
                        frames.len(),
                        1,
                        "Should return 1 placeholder frame for case: {}",
                        i
                    );
                }
            }
            _ => return Err(format!("Expected stackTrace response for case: {}", i).into()),
        }
    }
    Ok(())
}

#[test]
fn test_dap_scopes_edge_cases() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test scopes with various frame IDs
    let test_cases = vec![
        1,   // Valid frame
        0,   // Frame 0
        -1,  // Invalid negative
        999, // High frame number
    ];

    for frame_id in test_cases {
        let scope_args = json!({
            "frameId": frame_id
        });

        let response = adapter.handle_request(1, "scopes", Some(scope_args));
        match response {
            DapMessage::Response { success, command, body, .. } => {
                assert_eq!(command, "scopes");
                if success {
                    let body = body.ok_or("Expected body in successful response")?;
                    let scopes = body
                        .get("scopes")
                        .and_then(|s| s.as_array())
                        .ok_or("Expected scopes array")?;

                    // Should return at least one scope (Local) for any valid frame
                    if frame_id > 0 {
                        assert_eq!(scopes.len(), 3, "Should have 3 scopes for frame: {}", frame_id);

                        // Check scope structure
                        let scope = &scopes[0];
                        assert!(scope.get("name").is_some());
                        assert!(scope.get("variablesReference").is_some());
                        let scope_name = scope
                            .get("name")
                            .and_then(|n| n.as_str())
                            .ok_or("Expected scope name")?;
                        assert_eq!(scope_name, "Locals");
                    }
                }
            }
            _ => return Err(format!("Expected scopes response for frame: {}", frame_id).into()),
        }
    }
    Ok(())
}

#[test]
fn test_dap_pause_without_session() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test pause without active debugger session
    let response = adapter.handle_request(1, "pause", None);
    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "pause");
            assert!(!success, "Pause should fail without active session");
            if let Some(msg) = message {
                assert!(msg.contains("Failed to pause") || msg.contains("No active"));
            }
        }
        _ => return Err("Expected pause response".into()),
    }
    Ok(())
}

#[test]
fn test_dap_step_commands_without_session() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test all step commands without active session
    let step_commands = vec!["next", "stepIn", "stepOut", "continue"];

    for command in step_commands {
        let response = adapter.handle_request(1, command, None);
        match response {
            DapMessage::Response { success, command: resp_cmd, .. } => {
                assert_eq!(resp_cmd, *command);
                // These should succeed (they're graceful no-ops without session)
                assert!(success, "Step command {} should succeed gracefully", command);
            }
            _ => return Err(format!("Expected response for command: {}", command).into()),
        }
    }
    Ok(())
}

#[test]
fn test_dap_malformed_requests() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test various malformed or edge case requests
    let test_cases = [
        ("setBreakpoints", Some(json!({}))),             // Missing source
        ("setBreakpoints", Some(json!({"source": {}}))), // Missing path
        ("setBreakpoints", Some(json!({"source": {"path": ""}}))), // Empty path
        ("variables", None),                             // Missing variablesReference
        ("scopes", None),                                // Missing frameId
        ("evaluate", None),                              // Missing expression
        ("evaluate", Some(json!({"expression": ""}))),   // Empty expression
        ("stackTrace", Some(json!({"threadId": "not_a_number"}))), // Invalid thread ID type
    ];

    for (i, (command, args)) in test_cases.iter().enumerate() {
        let response = adapter.handle_request(i as i64 + 1, command, args.clone());
        match response {
            DapMessage::Response { success, command: resp_cmd, message, .. } => {
                assert_eq!(resp_cmd, *command);
                // Most should fail with helpful error messages
                if !success {
                    if let Some(msg) = message {
                        assert!(
                            !msg.is_empty(),
                            "Error message should not be empty for {}",
                            command
                        );
                    } else {
                        return Err(format!(
                            "Failed request should have error message for {}",
                            command
                        )
                        .into());
                    }
                }
            }
            _ => return Err(format!("Expected response for malformed command: {}", command).into()),
        }
    }
    Ok(())
}

#[test]
fn test_dap_attach_process_id_mode() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // PID attach should succeed in signal-control mode.
    let attach_args = json!({
        "processId": 12345
    });

    let response = adapter.handle_request(1, "attach", Some(attach_args));
    match response {
        DapMessage::Response { success, command, body, message, .. } => {
            assert_eq!(command, "attach");
            assert!(success, "PID attach should succeed");
            let body = body.ok_or("Expected attach body")?;
            assert_eq!(body.get("processId").and_then(|v| v.as_u64()), Some(12345));
            let msg = message.ok_or("Expected attach message")?;
            assert!(msg.contains("signal-control mode"));
        }
        _ => return Err("Expected attach response".into()),
    }
    Ok(())
}
