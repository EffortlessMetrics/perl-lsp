//! Real tests for Document Lifecycle Events
//! Tests didSave, willSave, and willSaveWaitUntil notifications

use perl_lsp::{JsonRpcRequest, JsonRpcResponse, LspServer};
use serde_json::{Value, json};

mod support;
use support::test_helpers::apply_text_edits;

/// Helper to set up an initialized LSP server with a document
fn setup_server_with_document() -> (LspServer, String) {
    let server = LspServer::new();

    // 1. Send initialize request with JsonRpcRequest
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((1) as i64)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    let init_response = server.handle_request(init_request);
    assert!(init_response.is_some(), "Initialize should return a response");

    // 2. CRITICAL: Send initialized notification
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None, // Notifications have no id
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);

    // 3. Open document
    let uri = "file:///test.pl".to_string();
    let open_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $foo = 42;\nuse strict;"
            }
        })),
    };
    server.handle_request(open_notification);

    (server, uri)
}

/// Helper to extract result from JsonRpcResponse
fn get_result(response: Option<JsonRpcResponse>) -> Option<Value> {
    response.and_then(|r| r.result)
}

#[test]
fn test_did_save_notification() {
    let (server, uri) = setup_server_with_document();

    // Send didSave notification
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None, // Notification has no id
        method: "textDocument/didSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 2
            }
        })),
    };
    let response = server.handle_request(request);

    // Notifications should not return a response
    assert!(response.is_none(), "Notifications should not return a response");
}

#[test]
fn test_did_save_with_text() {
    let (server, uri) = setup_server_with_document();

    // Send didSave notification with text
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 3
            },
            "text": "my $bar = 123;\nuse warnings;"
        })),
    };
    let response = server.handle_request(request);

    assert!(response.is_none(), "Notifications should not return a response");
}

#[test]
fn test_will_save_notification() {
    let (server, uri) = setup_server_with_document();

    // Send willSave notification - Manual save (reason = 1)
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/willSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri.clone()
            },
            "reason": 1 // Manual = 1
        })),
    };
    let response = server.handle_request(request);
    assert!(response.is_none(), "willSave notification should not return a response");

    // Test AfterDelay reason
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/willSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri.clone()
            },
            "reason": 2 // AfterDelay = 2
        })),
    };
    let response = server.handle_request(request);
    assert!(response.is_none());

    // Test FocusOut reason
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/willSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri
            },
            "reason": 3 // FocusOut = 3
        })),
    };
    let response = server.handle_request(request);
    assert!(response.is_none());
}

#[test]
fn test_will_save_wait_until_returns_valid_edits() -> Result<(), Box<dyn std::error::Error>> {
    let (server, uri) = setup_server_with_document();

    // Send willSaveWaitUntil request (this is a request, not notification)
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((10) as i64)),
        method: "textDocument/willSaveWaitUntil".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri
            },
            "reason": 1 // Manual
        })),
    };
    let response = server.handle_request(request);

    assert!(response.is_some(), "willSaveWaitUntil should return a response");
    let edits = get_result(response);
    assert!(edits.is_some(), "Response should have a result");

    let edits = edits.ok_or("Failed to get edits from response")?;
    assert!(edits.is_array(), "Response should be an array of text edits");

    // Native default formatting always returns a text-edit array. It may be
    // empty when the opened document is already outside the safe formatter
    // subset or needs no change.
    let edits_arr = edits.as_array().ok_or("Edits should be an array")?;
    for edit in edits_arr {
        assert!(edit.is_object());
        let obj = edit.as_object().ok_or("Edit should be an object")?;
        assert!(obj.contains_key("range"), "Edit must have range");
        assert!(obj.contains_key("newText"), "Edit must have newText");

        let range = &obj["range"];
        assert!(range["start"].is_object(), "Range start must be object");
        assert!(range["end"].is_object(), "Range end must be object");
    }

    Ok(())
}

#[test]
fn test_will_save_wait_until_with_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let (server, uri) = setup_server_with_document();

    // Update document with poorly formatted code
    let change_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didChange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri.clone(),
                "version": 2
            },
            "contentChanges": [{
                "text": "sub test{my$foo=42;return$foo;}\n"
            }]
        })),
    };
    server.handle_request(change_notification);

    // Send willSaveWaitUntil request
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((11) as i64)),
        method: "textDocument/willSaveWaitUntil".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri
            },
            "reason": 1 // Manual
        })),
    };
    let response = server.handle_request(request);

    assert!(response.is_some(), "willSaveWaitUntil should return a response");
    let edits = get_result(response);
    assert!(edits.is_some(), "Response should have a result");

    let edits = edits.ok_or("Failed to get edits from response")?;
    assert!(edits.is_array(), "Response should be an array of text edits");

    let edits_arr = edits.as_array().ok_or("Edits should be an array")?;
    assert!(
        !edits_arr.is_empty(),
        "native willSaveWaitUntil formatting should return edits for supported messy code"
    );
    let formatted = apply_text_edits("sub test{my$foo=42;return$foo;}\n", edits_arr);
    assert_eq!(
        formatted,
        concat!("sub test {\n", "    my $foo = 42;\n", "    return $foo;\n", "}\n", "\n",)
    );

    Ok(())
}

#[test]
fn test_did_close_after_save() {
    let (server, uri) = setup_server_with_document();

    // Send didSave notification
    let save_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri.clone(),
                "version": 2
            }
        })),
    };
    server.handle_request(save_notification);

    // Send didClose notification
    let close_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didClose".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri
            }
        })),
    };
    let response = server.handle_request(close_notification);

    assert!(response.is_none(), "didClose notification should not return a response");
}

#[test]
fn test_save_events_sequence() {
    let (server, uri) = setup_server_with_document();

    // Simulate typical save sequence: willSave -> willSaveWaitUntil -> didSave

    // 1. willSave notification
    let will_save = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/willSave".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri.clone() },
            "reason": 1
        })),
    };
    server.handle_request(will_save);

    // 2. willSaveWaitUntil request
    let will_save_wait_until = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((20) as i64)),
        method: "textDocument/willSaveWaitUntil".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri.clone() },
            "reason": 1
        })),
    };
    let edits_response = server.handle_request(will_save_wait_until);
    assert!(edits_response.is_some());

    // 3. didSave notification
    let did_save = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 3
            }
        })),
    };
    server.handle_request(did_save);

    // If we reach here without panics, the sequence completed successfully
}

#[test]
fn test_save_with_invalid_uri() {
    let (server, _uri) = setup_server_with_document();

    // Try to save a document that was never opened
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didSave".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///nonexistent.pl",
                "version": 1
            }
        })),
    };

    // Should handle gracefully without crashing
    let _response = server.handle_request(request);
    // Just checking it doesn't panic - the server may or may not return a response
}
