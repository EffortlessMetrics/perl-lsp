use super::*;
use crate::protocol::InlineValuesResponseBody;
use perl_tdd_support::must;
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

/// Create a temp file with valid Perl code for testing breakpoints.
/// NOTE: Avoid sub immediately followed by for loop (triggers parser hang - known issue)
fn create_test_perl_file() -> (NamedTempFile, String) {
    let mut file = must(NamedTempFile::with_suffix(".pl"));
    let perl_code = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
my $y = 2;
my $z = $x + $y;

if ($x > 0) {
    print "positive\n";
}

my @arr = (1, 2, 3);
while (my $item = shift @arr) {
    my $doubled = $item * 2;
    print "$doubled\n";
}

sub process {
    my ($value) = @_;
    my $result = $value * 2;
    return $result;
}

print "done\n";
my $final = process($x);
print "result: $final\n";
"#;
    must(file.write_all(perl_code.as_bytes()));
    must(file.flush());
    let path = file.path().to_string_lossy().to_string();
    (file, path)
}

#[test]
fn test_dispatcher_new() {
    let dispatcher = DapDispatcher::new();
    let breakpoints = dispatcher.breakpoint_store.get_breakpoints("/test.pl");
    assert_eq!(breakpoints.len(), 0);
}

#[test]
fn test_handle_initialize() -> Result<(), Box<dyn std::error::Error>> {
    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: Some(json!({
            "clientId": "vscode",
            "clientName": "Visual Studio Code",
            "adapterId": "perl-rs",
            "linesStartAt1": true,
            "columnsStartAt1": true,
        })),
    };

    let response = dispatcher.dispatch(&request);

    if !response.success {
        eprintln!("Error: {:?}", response.message);
    }
    assert!(response.success);
    assert_eq!(response.command, "initialize");
    assert!(response.body.is_some());

    // Parse capabilities
    let capabilities: Capabilities = serde_json::from_value(response.body.ok_or("Expected body")?)?;
    assert_eq!(capabilities.supports_configuration_done_request, Some(true));
    assert_eq!(capabilities.supports_evaluate_for_hovers, Some(true));
    Ok(())
}

#[test]
fn test_handle_set_breakpoints() -> Result<(), Box<dyn std::error::Error>> {
    let (_file, source_path) = create_test_perl_file();
    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "setBreakpoints".to_string(),
        arguments: Some(json!({
            "source": {
                "path": source_path,
                "name": "script.pl"
            },
            "breakpoints": [
                { "line": 10, "column": 0 },
                { "line": 25, "column": 0 }
            ]
        })),
    };

    let response = dispatcher.dispatch(&request);

    assert!(response.success);
    assert_eq!(response.command, "setBreakpoints");
    assert!(response.body.is_some());

    // Parse response body
    let body: SetBreakpointsResponseBody =
        serde_json::from_value(response.body.ok_or("Expected body")?)?;
    assert_eq!(body.breakpoints.len(), 2);
    assert_eq!(body.breakpoints[0].line, 10);
    assert_eq!(body.breakpoints[1].line, 25);
    assert!(body.breakpoints[0].verified);
    assert!(body.breakpoints[1].verified);
    Ok(())
}

#[test]
fn test_handle_set_breakpoints_replace_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let (_file, source_path) = create_test_perl_file();
    let dispatcher = DapDispatcher::new();

    // Set initial breakpoints
    let request1 = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "setBreakpoints".to_string(),
        arguments: Some(json!({
            "source": { "path": &source_path },
            "breakpoints": [{ "line": 10 }]
        })),
    };
    dispatcher.dispatch(&request1);

    // Replace with new breakpoints
    let request2 = Request {
        seq: 3,
        msg_type: "request".to_string(),
        command: "setBreakpoints".to_string(),
        arguments: Some(json!({
            "source": { "path": &source_path },
            "breakpoints": [{ "line": 20 }, { "line": 26 }]
        })),
    };
    let response = dispatcher.dispatch(&request2);

    assert!(response.success);
    let body: SetBreakpointsResponseBody =
        serde_json::from_value(response.body.ok_or("Expected body")?)?;
    assert_eq!(body.breakpoints.len(), 2);
    assert_eq!(body.breakpoints[0].line, 20);
    assert_eq!(body.breakpoints[1].line, 26);
    Ok(())
}

#[test]
fn test_handle_set_breakpoints_preserves_order() -> Result<(), Box<dyn std::error::Error>> {
    let (_file, source_path) = create_test_perl_file();
    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "setBreakpoints".to_string(),
        arguments: Some(json!({
            "source": { "path": &source_path },
            // Use lines within our 27-line test file, but out of order
            "breakpoints": [
                { "line": 25 },
                { "line": 10 },
                { "line": 15 }
            ]
        })),
    };

    let response = dispatcher.dispatch(&request);

    assert!(response.success);
    let body: SetBreakpointsResponseBody =
        serde_json::from_value(response.body.ok_or("Expected body")?)?;

    // Order must match request
    assert_eq!(body.breakpoints[0].line, 25);
    assert_eq!(body.breakpoints[1].line, 10);
    assert_eq!(body.breakpoints[2].line, 15);
    Ok(())
}

#[test]
fn test_handle_unknown_command() -> Result<(), Box<dyn std::error::Error>> {
    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 99,
        msg_type: "request".to_string(),
        command: "unknownCommand".to_string(),
        arguments: None,
    };

    let response = dispatcher.dispatch(&request);

    assert!(!response.success);
    assert_eq!(response.command, "unknownCommand");
    assert!(response.message.is_some());
    assert!(response
        .message
        .ok_or("Expected message")?
        .contains("Unknown command: unknownCommand"));
    Ok(())
}

#[test]
fn test_handle_set_breakpoints_missing_arguments() {
    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "setBreakpoints".to_string(),
        arguments: None,
    };

    let response = dispatcher.dispatch(&request);

    assert!(!response.success);
    assert!(response.message.is_some());
}

#[test]
fn test_handle_inline_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = NamedTempFile::with_suffix(".pl")?;
    let perl_code = "my $x = 1;\nmy $y = $x + 2;\n";
    file.write_all(perl_code.as_bytes())?;
    file.flush()?;
    let path = file.path().to_string_lossy().to_string();

    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 3,
        msg_type: "request".to_string(),
        command: "inlineValues".to_string(),
        arguments: Some(json!({
            "source": { "path": path },
            "startLine": 1,
            "endLine": 2
        })),
    };

    let response = dispatcher.dispatch(&request);
    assert!(response.success);

    let body: InlineValuesResponseBody =
        serde_json::from_value(response.body.ok_or("Expected body")?)?;
    assert!(body.inline_values.iter().any(|v| v.text.contains("$x")));
    assert!(body.inline_values.iter().any(|v| v.text.contains("$y")));
    Ok(())
}

#[test]
fn test_response_sequence_numbers() {
    let dispatcher = DapDispatcher::new();

    let request1 = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: None,
    };
    let response1 = dispatcher.dispatch(&request1);

    let request2 = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: None,
    };
    let response2 = dispatcher.dispatch(&request2);

    // Response sequence numbers should increment
    assert_eq!(response1.seq, 1);
    assert_eq!(response2.seq, 2);
}

#[test]
fn test_initialize_emits_initialized_event() {
    let dispatcher = DapDispatcher::new();
    let request = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: Some(json!({
            "clientId": "vscode",
            "adapterId": "perl-rs",
        })),
    };

    let result = dispatcher.dispatch_with_events(&request);

    // Response should be successful
    assert!(result.response.success);
    assert_eq!(result.response.command, "initialize");

    // Should emit initialized event
    assert_eq!(result.events.len(), 1);
    let event = &result.events[0];
    assert_eq!(event.event, "initialized");
    assert_eq!(event.msg_type, "event");
    assert!(event.body.is_none()); // initialized event has no body
}

#[test]
fn test_configuration_done_after_initialize() {
    let dispatcher = DapDispatcher::new();

    // First, initialize
    let init_request = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: None,
    };
    let init_result = dispatcher.dispatch_with_events(&init_request);
    assert!(init_result.response.success);

    // Then, configurationDone should succeed
    let config_done_request = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "configurationDone".to_string(),
        arguments: None,
    };
    let response = dispatcher.dispatch(&config_done_request);

    assert!(response.success);
    assert_eq!(response.command, "configurationDone");
}

#[test]
fn test_configuration_done_before_initialize_fails() -> Result<(), Box<dyn std::error::Error>> {
    let dispatcher = DapDispatcher::new();

    // configurationDone without initialize should fail
    let request = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "configurationDone".to_string(),
        arguments: None,
    };
    let response = dispatcher.dispatch(&request);

    assert!(!response.success);
    assert!(response.message.is_some());
    assert!(response.message.ok_or("Expected message")?.contains("before initialized"));
    Ok(())
}

#[test]
fn test_event_sequence_numbers() {
    let dispatcher = DapDispatcher::new();

    // Multiple initializations should have incrementing event seq numbers
    let request1 = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: None,
    };
    let result1 = dispatcher.dispatch_with_events(&request1);

    let request2 = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: None,
    };
    let result2 = dispatcher.dispatch_with_events(&request2);

    // Event sequence numbers should increment
    assert_eq!(result1.events[0].seq, 1);
    assert_eq!(result2.events[0].seq, 2);
}

#[test]
fn test_failed_initialize_no_event() {
    let dispatcher = DapDispatcher::new();

    // Invalid arguments that cause parsing to fail
    let request = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: Some(json!({
            "adapterId": 123 // Should be string, not number
        })),
    };

    let result = dispatcher.dispatch_with_events(&request);

    // If initialization fails, no event should be emitted
    if !result.response.success {
        assert!(result.events.is_empty());
    }
}
