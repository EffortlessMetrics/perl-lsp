//! Tests for LSP inline completion support

use perl_parser::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
fn test_inline_completion_after_arrow() {
    let mut server = LspServer::new();

    // Open a document
    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $obj = Package->"
            }
        })),
    };
    let _ = server.handle_request(open_request);

    // Request inline completions after ->
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/inlineCompletion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 19 }
        })),
    };
    let result = server.handle_request(request).unwrap();

    let items = result.result.unwrap();
    assert!(items.get("items").is_some());
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should suggest new()
    let first = &items[0];
    assert_eq!(first["insertText"].as_str().unwrap(), "new()");
}

#[test]
fn test_inline_completion_after_use() {
    let mut server = LspServer::new();

    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "use "
            }
        })),
    };
    let _ = server.handle_request(open_request);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/inlineCompletion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }
        })),
    };
    let result = server.handle_request(request).unwrap();

    let items = result.result.unwrap();
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should include strict and warnings
    let suggestions: Vec<String> = items
        .iter()
        .map(|i| i["insertText"].as_str().unwrap().to_string())
        .collect();
    assert!(suggestions.contains(&"strict;".to_string()));
    assert!(suggestions.contains(&"warnings;".to_string()));
}

#[test]
fn test_inline_completion_shebang() {
    let mut server = LspServer::new();

    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "#!"
            }
        })),
    };
    let _ = server.handle_request(open_request);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/inlineCompletion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 2 }
        })),
    };
    let result = server.handle_request(request).unwrap();

    let items = result.result.unwrap();
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should suggest shebang
    let first = &items[0];
    assert_eq!(first["insertText"].as_str().unwrap(), "/usr/bin/env perl");
}

#[test]
fn test_inline_completion_sub_body() {
    let mut server = LspServer::new();

    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "sub test "
            }
        })),
    };
    let _ = server.handle_request(open_request);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/inlineCompletion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 9 }
        })),
    };
    let result = server.handle_request(request).unwrap();

    let items = result.result.unwrap();
    let items = items["items"].as_array().unwrap();
    assert!(!items.is_empty());

    // Should suggest opening brace
    let first = &items[0];
    assert!(first["insertText"].as_str().unwrap().contains("{"));
}

#[test]
fn test_inline_completion_no_suggestions() {
    let mut server = LspServer::new();

    let uri = "file:///test.pl";
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $x = 42;"
            }
        })),
    };
    let _ = server.handle_request(open_request);

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/inlineCompletion".to_string(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 10 }
        })),
    };
    let result = server.handle_request(request).unwrap();

    let items = result.result.unwrap();
    let items = items["items"].as_array().unwrap();
    // Should have no suggestions in middle of statement
    assert!(items.is_empty());
}

