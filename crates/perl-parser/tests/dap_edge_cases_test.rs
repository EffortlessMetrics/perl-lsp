//! Enhanced edge case tests for DAP debugger output parsing
//!
//! These tests cover various edge cases and complex scenarios for the Perl debugger
//! output parsing to ensure robustness.

use perl_parser::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;
use std::sync::mpsc::channel;
// use std::time::Duration; // Unused import
use tempfile::tempdir;

/// Create a test Perl script with specific content
fn create_edge_case_script(content: &str) -> std::path::PathBuf {
    let dir = tempdir().unwrap();
    let script_path = dir.path().join("edge_case.pl");
    write(&script_path, content).unwrap();
    script_path
}

#[test]
fn test_dap_complex_perl_syntax() {
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

    let script_path = create_edge_case_script(complex_script);
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    // Test that we can handle complex breakpoint scenarios
    let bp_args = json!({
        "source": {"path": script_path.to_str().unwrap()},
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
            let body = body.unwrap();
            let breakpoints = body.get("breakpoints").and_then(|b| b.as_array()).unwrap();
            assert_eq!(breakpoints.len(), 3);

            // All breakpoints should be present (verified depends on session)
            for bp in breakpoints {
                assert!(bp.get("line").is_some());
                assert!(bp.get("id").is_some());
            }
        }
        _ => panic!("Expected successful setBreakpoints response"),
    }
}

#[test]
fn test_dap_evaluate_complex_expressions() {
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
                    assert!(message.is_some());
                    let msg = message.unwrap();
                    assert!(msg.contains("debugger session") || msg.contains("session"));
                }
            }
            _ => panic!("Expected evaluate response for expression: {}", expr),
        }
    }
}

#[test]
fn test_dap_variables_complex_scopes() {
    let mut adapter = DebugAdapter::new();

    // Test different variable reference scenarios
    let test_cases = vec![
        1,   // Local scope
        2,   // Different frame
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
                    let body = body.unwrap();
                    let variables = body.get("variables").and_then(|v| v.as_array()).unwrap();
                    // Should return some variables for valid references
                    if var_ref == 1 {
                        assert!(!variables.is_empty(), "Local scope should have default variables");
                    }
                } else {
                    // Invalid references should fail gracefully
                    if var_ref <= 0 || var_ref > 100 {
                        // Expected for invalid references
                    }
                }
            }
            _ => panic!("Expected variables response for reference: {}", var_ref),
        }
    }
}

#[test]
fn test_dap_stack_trace_edge_cases() {
    let mut adapter = DebugAdapter::new();

    // Test stack trace with various thread IDs and scenarios
    let test_cases = vec![
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
                    let body = body.unwrap();
                    let frames = body.get("stackFrames").and_then(|f| f.as_array()).unwrap();
                    let total = body.get("totalFrames").and_then(|t| t.as_u64()).unwrap_or(0);

                    // Without active session, should return empty
                    assert_eq!(frames.len(), 0);
                    assert_eq!(total, 0);
                }
            }
            _ => panic!("Expected stackTrace response for case: {}", i),
        }
    }
}

#[test]
fn test_dap_scopes_edge_cases() {
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
                    let body = body.unwrap();
                    let scopes = body.get("scopes").and_then(|s| s.as_array()).unwrap();

                    // Should return at least one scope (Local) for any valid frame
                    if frame_id > 0 {
                        assert!(!scopes.is_empty(), "Should have scopes for frame: {}", frame_id);

                        // Check scope structure
                        let scope = &scopes[0];
                        assert!(scope.get("name").is_some());
                        assert!(scope.get("variablesReference").is_some());
                        assert_eq!(scope.get("name").and_then(|n| n.as_str()).unwrap(), "Local");
                    }
                }
            }
            _ => panic!("Expected scopes response for frame: {}", frame_id),
        }
    }
}

#[test]
fn test_dap_pause_without_session() {
    let mut adapter = DebugAdapter::new();

    // Test pause without active debugger session
    let response = adapter.handle_request(1, "pause", None);
    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "pause");
            assert!(!success, "Pause should fail without active session");
            assert!(message.is_some());
            let msg = message.unwrap();
            assert!(msg.contains("Failed to pause") || msg.contains("No active"));
        }
        _ => panic!("Expected pause response"),
    }
}

#[test]
fn test_dap_step_commands_without_session() {
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
            _ => panic!("Expected response for command: {}", command),
        }
    }
}

#[test]
fn test_dap_malformed_requests() {
    let mut adapter = DebugAdapter::new();

    // Test various malformed or edge case requests
    let test_cases = vec![
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
                    assert!(
                        message.is_some(),
                        "Failed request should have error message for {}",
                        command
                    );
                    let msg = message.unwrap();
                    assert!(!msg.is_empty(), "Error message should not be empty for {}", command);
                }
            }
            _ => panic!("Expected response for malformed command: {}", command),
        }
    }
}

#[test]
fn test_dap_attach_not_implemented() {
    let mut adapter = DebugAdapter::new();

    // Test attach command which is not yet implemented
    let attach_args = json!({
        "processId": 12345
    });

    let response = adapter.handle_request(1, "attach", Some(attach_args));
    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "attach");
            assert!(!success, "Attach should not be implemented yet");
            assert!(message.is_some());
            let msg = message.unwrap();
            assert!(msg.contains("not yet implemented") || msg.contains("not implemented"));
        }
        _ => panic!("Expected attach response"),
    }
}
