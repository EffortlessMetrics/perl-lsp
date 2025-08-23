use perl_parser::lsp_server::{LspServer, JsonRpcRequest};
use serde_json::{json, Value};

/// Test that ensures LSP capabilities match GA contract
/// This prevents accidental drift where we advertise features that don't work
#[test]
fn test_ga_capabilities_contract() {
    let mut server = LspServer::new();
    
    // Send initialize request through public API
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///tmp/test",
            "capabilities": {}
        })),
    };
    
    let response = server.handle_request(request).unwrap();
    assert!(response.error.is_none(), "Initialize should succeed");
    let caps = response.result.unwrap()["capabilities"].clone();
    
    // Assert what SHOULD be advertised (working features)
    assert!(caps["textDocumentSync"].is_object(), "textDocumentSync must be advertised");
    assert!(caps["completionProvider"].is_object(), "completionProvider must be advertised");
    assert_eq!(caps["hoverProvider"], json!(true), "hoverProvider must be true");
    assert_eq!(caps["definitionProvider"], json!(true), "definitionProvider must be true");
    assert_eq!(caps["declarationProvider"], json!(true), "declarationProvider must be true");
    assert_eq!(caps["referencesProvider"], json!(true), "referencesProvider must be true");
    assert_eq!(caps["documentHighlightProvider"], json!(true), "documentHighlightProvider must be true");
    assert!(caps["signatureHelpProvider"].is_object(), "signatureHelpProvider must be advertised");
    assert_eq!(caps["documentSymbolProvider"], json!(true), "documentSymbolProvider must be true");
    assert_eq!(caps["foldingRangeProvider"], json!(true), "foldingRangeProvider must be true");
    
    // Assert what SHOULD NOT be advertised (stubs/partial implementations)
    assert!(caps["renameProvider"].is_null(), "renameProvider must NOT be advertised (stub)");
    assert!(caps["codeActionProvider"].is_null(), "codeActionProvider must NOT be advertised (partial)");
    assert!(caps["workspaceSymbolProvider"].is_null(), "workspaceSymbolProvider must NOT be advertised (cross-file not wired)");
    assert!(caps["codeLensProvider"].is_null(), "codeLensProvider must NOT be advertised (partial)");
    assert!(caps["semanticTokensProvider"].is_null(), "semanticTokensProvider must NOT be advertised (partial)");
    assert!(caps["inlayHintProvider"].is_null(), "inlayHintProvider must NOT be advertised (partial)");
    assert!(caps["typeHierarchyProvider"].is_null(), "typeHierarchyProvider must NOT be advertised (not implemented)");
    assert!(caps["callHierarchyProvider"].is_null(), "callHierarchyProvider must NOT be advertised (partial)");
    assert!(caps["documentLinkProvider"].is_null(), "documentLinkProvider must NOT be advertised (stub)");
    assert!(caps["selectionRangeProvider"].is_null(), "selectionRangeProvider must NOT be advertised (stub)");
    assert!(caps["documentOnTypeFormattingProvider"].is_null(), "documentOnTypeFormattingProvider must NOT be advertised (unreliable)");
    assert!(caps["executeCommandProvider"].is_null(), "executeCommandProvider must NOT be advertised (not wired)");
    
    // documentFormattingProvider is conditional on perltidy availability - that's OK
    // It can be either true or null depending on environment
}

/// Test that unsupported methods return proper errors
#[test]
fn test_unsupported_methods_return_error() {
    let mut server = LspServer::new();
    
    // Initialize first through public API
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(0)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///tmp/test",
            "capabilities": {}
        })),
    };
    server.handle_request(init_request);
    
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: None,
    };
    server.handle_request(initialized_request);
    
    // Test that unsupported methods return method_not_found error
    let unsupported_methods = [
        "textDocument/rename",
        "textDocument/codeAction", 
        "textDocument/codeLens",
        "textDocument/inlayHint",
        "textDocument/semanticTokens/full",
        "textDocument/semanticTokens/range",
        "textDocument/typeDefinition",
        "textDocument/implementation",
        "workspace/symbol",
        "workspace/executeCommand",
    ];
    
    for method in &unsupported_methods {
        let request = perl_parser::lsp_server::JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: method.to_string(),
            params: Some(json!({})),
        };
        
        let response = server.handle_request(request);
        assert!(response.is_some(), "Method {} should return a response", method);
        
        let resp = response.unwrap();
        assert!(resp.error.is_some(), "Method {} should return an error", method);
        assert_eq!(resp.error.unwrap().code, -32601, "Method {} should return method_not_found error", method);
    }
}