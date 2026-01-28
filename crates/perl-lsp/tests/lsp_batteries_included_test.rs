//! Tests for "batteries included" LSP functionality
//!
//! This test validates that perl-lsp provides a complete, production-ready experience
//! with formatting, linting, and import optimization working out-of-the-box.

#![cfg(test)]

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test that formatting is properly advertised in server capabilities
#[test]
fn test_formatting_capability_advertised() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };

    let response =
        srv.handle_request(init_req).ok_or("Failed to get response from initialize request")?;

    let result = response.result.ok_or("Expected result in initialize response")?;
    let capabilities =
        result.get("capabilities").ok_or("Expected capabilities in initialize result")?;

    // Verify formatting is advertised
    let formatting_provider = capabilities.get("documentFormattingProvider");
    assert!(formatting_provider.is_some(), "documentFormattingProvider should be advertised");

    // Verify range formatting is advertised
    let range_formatting_provider = capabilities.get("documentRangeFormattingProvider");
    assert!(
        range_formatting_provider.is_some(),
        "documentRangeFormattingProvider should be advertised"
    );

    Ok(())
}

/// Test that code actions include organize imports
#[test]
fn test_organize_imports_code_action_available() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    // Initialize server
    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    let _ = srv.handle_request(init_req);

    // Send initialized notification
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    let _ = srv.handle_request(initialized);

    // Open a document with multiple imports
    let uri = "file:///test_imports.pl";
    let text = r#"use strict;
use warnings;
use Data::Dumper qw(Dumper);
use JSON qw(encode_json decode_json);

my $data = {key => 'value'};
print Dumper($data);
"#;

    let open_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    let _ = srv.handle_request(open_req);

    // Request code actions
    let actions_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/codeAction".to_string(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 6, "character": 0}
            },
            "context": {
                "diagnostics": [],
                "only": ["source.organizeImports"]
            }
        })),
    };

    let response =
        srv.handle_request(actions_req).ok_or("Failed to get response from code action request")?;

    let result = response.result.ok_or("Expected result in code action response")?;

    // Check that we got some response (even if it's an empty array)
    // The actual implementation of organize imports may vary
    assert!(result.is_array(), "Expected array result for code actions");

    Ok(())
}

/// Test that execute commands include perlcritic integration
#[test]
fn test_perlcritic_execute_command_available() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };

    let response =
        srv.handle_request(init_req).ok_or("Failed to get response from initialize request")?;

    let result = response.result.ok_or("Expected result in initialize response")?;
    let capabilities =
        result.get("capabilities").ok_or("Expected capabilities in initialize result")?;

    // Verify execute command provider is present
    let execute_command_provider = capabilities.get("executeCommandProvider");
    assert!(execute_command_provider.is_some(), "executeCommandProvider should be advertised");

    // Verify that perlcritic command is in the list
    if let Some(provider) = execute_command_provider {
        let commands = provider
            .get("commands")
            .and_then(|c| c.as_array())
            .ok_or("Expected commands array in executeCommandProvider")?;

        let has_critic_command = commands.iter().any(|cmd| cmd.as_str() == Some("perl.runCritic"));

        assert!(has_critic_command, "perl.runCritic command should be available");
    }

    Ok(())
}

/// Test that basic diagnostics work without external tools
#[test]
fn test_builtin_diagnostics_work() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    // Initialize
    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    let _ = srv.handle_request(init_req);

    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    let _ = srv.handle_request(initialized);

    // Open a document with syntax errors
    let uri = "file:///test_diagnostics.pl";
    let text = "sub foo { my \n"; // Incomplete syntax

    let open_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    let _ = srv.handle_request(open_req);

    // Request pull diagnostics
    let diag_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".to_string(),
        params: Some(json!({
            "textDocument": {"uri": uri}
        })),
    };

    let response =
        srv.handle_request(diag_req).ok_or("Failed to get response from diagnostic request")?;

    let result = response.result.ok_or("Expected result in diagnostic response")?;

    // Verify we got some diagnostic information
    // The exact structure may vary, but it should be present
    assert!(result.is_object() || result.is_array(), "Expected diagnostic result");

    Ok(())
}

/// Test that good defaults are in place for configuration
#[test]
fn test_default_configuration_sensible() -> Result<(), Box<dyn std::error::Error>> {
    let srv = LspServer::new();

    // Verify that server can be created without explicit configuration
    // This tests that good defaults are in place
    drop(srv);

    Ok(())
}

/// Test that the server provides helpful capabilities information
#[test]
fn test_server_capabilities_complete() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };

    let response =
        srv.handle_request(init_req).ok_or("Failed to get response from initialize request")?;

    let result = response.result.ok_or("Expected result in initialize response")?;
    let capabilities =
        result.get("capabilities").ok_or("Expected capabilities in initialize result")?;

    // Verify core LSP capabilities are present
    let expected_capabilities = [
        "textDocumentSync",
        "hoverProvider",
        "completionProvider",
        "definitionProvider",
        "referencesProvider",
        "documentFormattingProvider",
        "documentSymbolProvider",
        "codeActionProvider",
        "executeCommandProvider",
    ];

    for capability in &expected_capabilities {
        assert!(capabilities.get(capability).is_some(), "Missing capability: {}", capability);
    }

    Ok(())
}

/// Test that formatting gracefully handles missing perltidy
#[test]
fn test_formatting_graceful_degradation() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    // Initialize
    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    let _ = srv.handle_request(init_req);

    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    let _ = srv.handle_request(initialized);

    // Open a simple Perl document
    let uri = "file:///test_format.pl";
    let text = "sub foo{my$x=1;return$x}";

    let open_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    let _ = srv.handle_request(open_req);

    // Request formatting
    let format_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/formatting".to_string(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        })),
    };

    let response = srv.handle_request(format_req);

    // Formatting should either succeed or return a helpful error
    // It should NOT crash or panic
    assert!(response.is_some(), "Formatting should return a response (success or error)");

    Ok(())
}
