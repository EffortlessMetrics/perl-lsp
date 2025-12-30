//! Tests for LSP execute command functionality
use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

fn setup_server() -> LspServer {
    let mut server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootPath": "/test",
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    let _response = server.handle_request(init_request);

    // Send the initialized notification to complete the handshake
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialized".to_string(),
        params: Some(json!({})),
        id: None,
    };

    let _initialized_response = server.handle_request(initialized_request);
    server
}

#[test]

fn test_execute_command_run_file() {
    let mut server = setup_server();

    // Create a test file
    let test_content = r#"#!/usr/bin/perl
use strict;
use warnings;

print "Hello, World!\n";
"#;

    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": test_content
            }
        })),
        id: None,
    };

    // Send the notification
    let _ = server.handle_request(open_request);

    // Execute the run file command
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.runFile",
            "arguments": ["/test.pl"]
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(execute_request).unwrap();
    let result = response.result.unwrap();

    // Check that we got a response (even if the command might fail due to file not existing)
    assert!(result.is_object());
    assert!(result.get("success").is_some());
    assert!(result.get("output").is_some());
}

#[test]

fn test_execute_command_run_tests() {
    let mut server = setup_server();

    // Create a test file with Test::More
    let test_content = r#"#!/usr/bin/perl
use strict;
use warnings;
use Test::More tests => 2;

ok(1, "First test");
is(1 + 1, 2, "Math works");
"#;

    let uri = "file:///test.t";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": test_content
            }
        })),
        id: None,
    };

    // Send the notification
    let _ = server.handle_request(open_request);

    // Execute the run tests command
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.runTests",
            "arguments": ["/test.t"]
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(execute_request).unwrap();
    let result = response.result.unwrap();

    // Check response structure
    assert!(result.is_object());
    assert!(result.get("success").is_some());
    assert!(result.get("output").is_some());

    // Check that it recognized this as a test file
    if result.get("command").is_some() {
        let command = result.get("command").unwrap().as_str().unwrap();
        // If prove is available, it should use prove for .t files
        assert!(command == "prove" || command == "perl");
    }
}

#[test]

fn test_execute_command_unknown() {
    let mut server = setup_server();

    // Try an unknown command
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.unknownCommand",
            "arguments": []
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(execute_request);

    // Should return an error
    assert!(response.is_some());
    let response = response.unwrap();
    assert!(response.error.is_some());

    // The error is serialized to JSON so we need to check it differently
    // For now, just verify we got an error
    // The specific error code and message are implementation details
}

#[test]

fn test_execute_command_capabilities() {
    let mut server = LspServer::new();

    // Initialize and check capabilities
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootPath": "/test",
            "capabilities": {}
        })),
        id: Some(json!(10)),
    };

    let response = server.handle_request(init_request).unwrap();
    let result = response.result.unwrap();
    let capabilities = result.get("capabilities").unwrap();
    let execute_command = capabilities.get("executeCommandProvider").unwrap();
    let commands = execute_command.get("commands").unwrap().as_array().unwrap();

    // Check that our new commands are advertised
    let command_strs: Vec<&str> = commands.iter().filter_map(|v| v.as_str()).collect();

    assert!(command_strs.contains(&"perl.runTests"));
    assert!(command_strs.contains(&"perl.runFile"));
    assert!(command_strs.contains(&"perl.runTestSub"));
    assert!(command_strs.contains(&"perl.debugTests"));
}
