//! Tests for LSP execute command functionality
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn setup_server(root_path: Option<String>) -> LspServer {
    let server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootPath": root_path,
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
fn test_execute_command_run_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path.clone()));

    // Create a test file
    let test_content = r#"#!/usr/bin/perl
use strict;
use warnings;

print "Hello, World!\n";
"#;

    let file_path = temp_dir.path().join("test.pl");
    fs::write(&file_path, test_content)?;
    let file_path_str = file_path.to_string_lossy().to_string();

    let uri = format!("file://{}", file_path_str);
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
            "arguments": [file_path_str]
        })),
        id: Some(json!(2)),
    };

    let response =
        server.handle_request(execute_request).ok_or("No response from execute command")?;
    let result = response.result.ok_or("No result in response")?;

    // Check that we got a response (even if the command might fail due to perl not installed/env issues)
    assert!(result.is_object());
    assert!(result.get("success").is_some());
    // output or error should be present
    assert!(result.get("output").is_some() || result.get("error").is_some());

    Ok(())
}

#[test]
fn test_execute_command_run_tests() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path.clone()));

    // Create a test file with Test::More
    let test_content = r#"#!/usr/bin/perl
use strict;
use warnings;
use Test::More tests => 2;

ok(1, "First test");
is(1 + 1, 2, "Math works");
"#;

    let file_path = temp_dir.path().join("test.t");
    fs::write(&file_path, test_content)?;
    let file_path_str = file_path.to_string_lossy().to_string();

    let uri = format!("file://{}", file_path_str);
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
            "arguments": [file_path_str]
        })),
        id: Some(json!(2)),
    };

    let response =
        server.handle_request(execute_request).ok_or("No response from execute command")?;
    let result = response.result.ok_or("No result in response")?;

    // Check response structure
    assert!(result.is_object());
    assert!(result.get("success").is_some());
    assert!(result.get("output").is_some());

    // Check that it recognized this as a test file
    if result.get("command").is_some() {
        let command = result
            .get("command")
            .ok_or("No command in result")?
            .as_str()
            .ok_or("Command is not a string")?;
        // If prove is available, it should use prove for .t files
        assert!(command == "prove" || command == "perl");
    }

    Ok(())
}

#[test]
fn test_execute_command_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server(None);

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
    let response = response.ok_or("Expected a response for unknown command")?;
    assert!(response.error.is_some());

    Ok(())
}

#[test]
fn test_execute_command_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

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

    let response = server.handle_request(init_request).ok_or("No response from initialize")?;
    let result = response.result.ok_or("No result in initialize response")?;
    let capabilities = result.get("capabilities").ok_or("No capabilities in result")?;
    let execute_command = capabilities
        .get("executeCommandProvider")
        .ok_or("No executeCommandProvider in capabilities")?;
    let commands = execute_command
        .get("commands")
        .ok_or("No commands in executeCommandProvider")?
        .as_array()
        .ok_or("Commands is not an array")?;

    // Check that our new commands are advertised
    let command_strs: Vec<&str> = commands.iter().filter_map(|v| v.as_str()).collect();

    assert!(command_strs.contains(&"perl.runTests"));
    assert!(command_strs.contains(&"perl.runFile"));
    assert!(command_strs.contains(&"perl.runTestSub"));
    assert!(command_strs.contains(&"perl.runCritic"));
    assert!(command_strs.contains(&"perl.explainProviderDecision"));
    assert!(command_strs.contains(&"perl.previewSafeDelete"));
    assert!(command_strs.contains(&"perl.previewPackageRename"));

    Ok(())
}

#[test]
fn test_execute_command_explain_provider_decision() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server(None);

    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.explainProviderDecision",
            "arguments": [{
                "provider": "goto_definition",
                "receipt_id": "semantic-shadow-compare",
                "scenario": "mojolicious-navigation",
                "request_receipt": {
                    "provider": "goto_definition",
                    "decision": "acted",
                    "fact_source": "compiler_fact",
                    "confidence": "high",
                    "freshness": "fresh"
                },
                "request_position": {
                    "uri_scheme": "file",
                    "line": 7,
                    "character": 2
                }
            }]
        })),
        id: Some(json!(2)),
    };

    let response = server
        .handle_request(execute_request)
        .ok_or("No response from explain-provider-decision command")?;
    let result = response.result.ok_or("No result in explain-provider-decision response")?;

    assert_eq!(result.get("provider").and_then(|value| value.as_str()), Some("goto_definition"));
    assert_eq!(result.get("decision").and_then(|value| value.as_str()), Some("acted"));
    assert_eq!(
        result.get("reason").and_then(|value| value.as_str()),
        Some("source_backed_high_confidence")
    );
    assert_eq!(result.get("fact_source").and_then(|value| value.as_str()), Some("compiler_fact"));
    assert_eq!(result.get("confidence").and_then(|value| value.as_str()), Some("high"));
    assert_eq!(result.get("freshness").and_then(|value| value.as_str()), Some("fresh"));
    assert_eq!(result.get("fallback").and_then(|value| value.as_str()), Some("none"));
    assert_eq!(
        result.get("receipt_id").and_then(|value| value.as_str()),
        Some("semantic-shadow-compare")
    );
    assert_eq!(
        result.get("scenario").and_then(|value| value.as_str()),
        Some("mojolicious-navigation")
    );
    let request_receipt = result
        .get("request_receipt")
        .and_then(|value| value.as_object())
        .ok_or("missing request_receipt")?;
    assert_eq!(
        request_receipt.get("provider").and_then(|value| value.as_str()),
        Some("goto_definition")
    );
    assert_eq!(
        request_receipt.get("fact_source").and_then(|value| value.as_str()),
        Some("compiler_fact")
    );
    assert_eq!(result.get("dynamic_boundary").and_then(|value| value.as_bool()), Some(false));
    let copyable_payload = result
        .get("copyable_payload")
        .and_then(|value| value.as_object())
        .ok_or("missing copyable_payload")?;
    assert_eq!(
        copyable_payload.get("schema_version").and_then(|value| value.as_str()),
        Some("provider_decision_bug_report.v1")
    );
    assert_eq!(
        copyable_payload.get("perl_lsp_version").and_then(|value| value.as_str()),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        copyable_payload.get("provider").and_then(|value| value.as_str()),
        Some("goto_definition")
    );
    assert_eq!(
        copyable_payload.get("support_tier_link").and_then(|value| value.as_str()),
        Some("docs/project/status/SUPPORT_TIERS.md#claim-rows")
    );
    let copyable_position = copyable_payload
        .get("request_position")
        .and_then(|value| value.as_object())
        .ok_or("missing copyable request_position")?;
    assert_eq!(copyable_position.get("uri_scheme").and_then(|value| value.as_str()), Some("file"));
    assert_eq!(copyable_position.get("line").and_then(|value| value.as_u64()), Some(7));
    assert_eq!(copyable_position.get("character").and_then(|value| value.as_u64()), Some(2));

    Ok(())
}
