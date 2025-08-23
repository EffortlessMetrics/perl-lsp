use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test Pull Diagnostics support (LSP 3.17)
#[test]
fn test_document_diagnostic() {
    let mut server = LspServer::new();
    
    // Initialize server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    let _ = server.handle_request(init_request);

    // Open a document with errors
    let uri = "file:///test.pl";
    let content = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
print $y;  # Undefined variable
"#;

    // Send didOpen notification
    let did_open_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    };
    let _ = server.handle_request(did_open_request);

    // Request diagnostics
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri }
        })),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();
    
    assert_eq!(result["kind"], "full");
    assert!(result["resultId"].is_string(), "Should have a result ID");
    
    let items = result["items"].as_array().unwrap();
    assert!(!items.is_empty(), "Should have at least one diagnostic");
    
    // Check first diagnostic structure
    let first = &items[0];
    assert!(first["range"].is_object());
    assert!(first["severity"].is_number());
    assert_eq!(first["source"], "perl-lsp");
    assert!(first["message"].is_string());
}

#[test]
fn test_document_diagnostic_unchanged() {
    let mut server = LspServer::new();
    
    // Initialize server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    let _ = server.handle_request(init_request);

    // Open a document
    let uri = "file:///test.pl";
    let content = r#"#!/usr/bin/perl
print "Hello, World!\n";
"#;

    let did_open_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    };
    let _ = server.handle_request(did_open_request);

    // First request - get full diagnostics
    let request1 = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri }
        })),
    };

    let response1 = server.handle_request(request1).unwrap();
    let result1 = response1.result.unwrap();
    let result_id = result1["resultId"].as_str().unwrap();

    // Second request with previous result ID - should be unchanged
    let request2 = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "previousResultId": result_id
        })),
    };

    let response2 = server.handle_request(request2).unwrap();
    let result2 = response2.result.unwrap();
    
    assert_eq!(result2["kind"], "unchanged");
    assert_eq!(result2["resultId"], result_id);
}

#[test]
fn test_workspace_diagnostic() {
    let mut server = LspServer::new();
    
    // Initialize server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    let _ = server.handle_request(init_request);

    // Open multiple documents
    let uri1 = "file:///test1.pl";
    let content1 = r#"use strict;
my $x = 1;
print $y;  # Error
"#;

    let uri2 = "file:///test2.pl";
    let content2 = r#"#!/usr/bin/perl
print "OK\n";
"#;

    // Open first document
    let did_open1 = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri1,
                "languageId": "perl",
                "version": 1,
                "text": content1
            }
        })),
    };
    let _ = server.handle_request(did_open1);

    // Open second document
    let did_open2 = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri2,
                "languageId": "perl",
                "version": 1,
                "text": content2
            }
        })),
    };
    let _ = server.handle_request(did_open2);

    // Request workspace diagnostics
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.unwrap();
    
    let items = result["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "Should have diagnostics for both documents");
    
    // Check structure of workspace diagnostic reports
    for item in items {
        assert!(item["uri"].is_string());
        assert!(item["version"].is_number());
        assert!(item["kind"].is_string());
        if item["kind"] == "full" {
            assert!(item["items"].is_array());
        }
    }
}

#[test]
fn test_diagnostic_provider_capability() {
    let mut server = LspServer::new();
    
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    
    let response = server.handle_request(init_request).unwrap();
    let result = response.result.unwrap();
    let caps = &result["capabilities"];
    
    // Diagnostic provider should be advertised in non-lock mode
    if !cfg!(feature = "lsp-ga-lock") {
        assert!(caps["diagnosticProvider"].is_object(), "diagnosticProvider should be advertised");
        let diag_provider = &caps["diagnosticProvider"];
        assert_eq!(diag_provider["interFileDependencies"], json!(false));
        assert_eq!(diag_provider["workspaceDiagnostics"], json!(true));
    }
}

#[test]
fn test_workspace_diagnostic_with_previous_ids() {
    let mut server = LspServer::new();
    
    // Initialize server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    let _ = server.handle_request(init_request);

    // Open a document
    let uri = "file:///test.pl";
    let content = r#"print "Hello\n";"#;

    let did_open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    };
    let _ = server.handle_request(did_open);

    // First workspace diagnostic request
    let request1 = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    };

    let response1 = server.handle_request(request1).unwrap();
    let result1 = response1.result.unwrap();
    let items1 = result1["items"].as_array().unwrap();
    
    // Get result IDs from first request
    let mut previous_ids = Vec::new();
    for item in items1 {
        if let Some(result_id) = item["resultId"].as_str() {
            previous_ids.push(json!({
                "uri": item["uri"],
                "value": result_id
            }));
        }
    }

    // Second request with previous IDs
    let request2 = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({
            "previousResultIds": previous_ids
        })),
    };

    let response2 = server.handle_request(request2).unwrap();
    let result2 = response2.result.unwrap();
    let items2 = result2["items"].as_array().unwrap();
    
    // At least one should be unchanged
    let has_unchanged = items2.iter().any(|item| item["kind"] == "unchanged");
    assert!(has_unchanged, "Should have at least one unchanged result");
}