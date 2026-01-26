#![cfg(test)]

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test that pragma code actions are offered
/// NOTE: Feature incomplete - workspace_roots() returns empty, so code actions don't include pragma suggestions
#[cfg(feature = "lsp-extras")]
#[test]
fn test_pragma_code_actions() -> Result<(), Box<dyn std::error::Error>> {
    let mut srv = LspServer::new();

    // Initialize server
    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {}
        })),
    };
    let _ = srv.handle_request(init_req);

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    let _ = srv.handle_request(initialized);

    // Open a document without pragmas
    let uri = "file:///test.pl";
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

    // Request code actions
    let actions_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/codeAction".to_string(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0}
            },
            "context": {"diagnostics": []}
        })),
    };

    let response = srv.handle_request(actions_req)
        .ok_or("Failed to get response from code action request")?;

    let result = response.result
        .ok_or("Expected result in code action response")?;
    let actions = result.as_array()
        .ok_or("Expected array of actions in code action result")?;

    // Look for pragma actions
    let has_strict_action = actions.iter().any(|a| a["title"].as_str() == Some("Add use strict;"));
    let has_warnings_action =
        actions.iter().any(|a| a["title"].as_str() == Some("Add use warnings;"));

    assert!(has_strict_action, "Expected 'Add use strict;' action");
    assert!(has_warnings_action, "Expected 'Add use warnings;' action");

    Ok(())
}

/// Test that formatting provider is advertised when perltidy is available
#[test]
fn test_formatting_provider_capability() -> Result<(), Box<dyn std::error::Error>> {
    let has_perltidy = perl_lsp::execute_command::command_exists("perltidy");

    let mut srv = LspServer::new();

    let init_req = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "capabilities": {}
        })),
    };

    let response = srv.handle_request(init_req)
        .ok_or("Failed to get response from initialize request")?;

    let result = response.result
        .ok_or("Expected result in initialize response")?;
    let has_formatting =
        result["capabilities"]["documentFormattingProvider"].as_bool().unwrap_or(false);

    // Formatting should only be advertised if perltidy is available
    assert_eq!(
        has_formatting, has_perltidy,
        "documentFormattingProvider should match perltidy availability"
    );

    Ok(())
}
