use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test Workspace Symbol Resolve support (LSP 3.17)
#[test]
fn test_workspace_symbol_resolve() {
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

    // Open a document with symbols
    let uri = "file:///test.pl";
    let content = r#"package MyModule;

sub hello {
    my $greeting = "Hello, World!";
    print $greeting;
}

our $VERSION = '1.0';
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

    // Create a basic symbol (as would be returned by workspace/symbol)
    let basic_symbol = json!({
        "name": "hello",
        "kind": 12, // Function
        "location": {
            "uri": uri,
            "range": {
                "start": { "line": 2, "character": 0 },
                "end": { "line": 6, "character": 1 }
            }
        }
    });

    // Request symbol resolution
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/symbol/resolve".into(),
        params: Some(basic_symbol.clone()),
    };

    let response = server.handle_request(request).unwrap();
    let resolved = response.result.unwrap();
    
    // Check that the symbol was enhanced
    assert_eq!(resolved["name"], "hello");
    assert!(resolved["detail"].is_string(), "Should have detail field");
    assert!(resolved["detail"].as_str().unwrap().contains("sub"), "Detail should indicate it's a subroutine");
    
    // Location should still be present
    assert!(resolved["location"].is_object());
    assert!(resolved["location"]["uri"].is_string());
    assert!(resolved["location"]["range"].is_object());
}

#[test]
fn test_workspace_symbol_resolve_with_container() {
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

    // Open a document with nested symbols
    let uri = "file:///test.pl";
    let content = r#"package Foo::Bar;

sub method_in_package {
    my $x = 1;
}

package Baz;

sub another_method {
    return 42;
}
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

    // Create a basic symbol for a method
    let basic_symbol = json!({
        "name": "method_in_package",
        "kind": 12, // Function
        "location": {
            "uri": uri,
            "range": {
                "start": { "line": 2, "character": 0 },
                "end": { "line": 4, "character": 1 }
            }
        }
    });

    // Request symbol resolution
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/symbol/resolve".into(),
        params: Some(basic_symbol),
    };

    let response = server.handle_request(request).unwrap();
    let resolved = response.result.unwrap();
    
    // Check enhanced fields
    assert_eq!(resolved["name"], "method_in_package");
    assert!(resolved["detail"].is_string());
    
    // Should potentially have container information
    // (depending on implementation details)
    if resolved["containerName"].is_string() {
        assert!(resolved["containerName"].as_str().unwrap().len() > 0);
    }
}

#[test]
fn test_workspace_symbol_resolve_capability() {
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
    
    // Workspace symbol provider should advertise resolve support in non-lock mode
    if !cfg!(feature = "lsp-ga-lock") {
        let ws_provider = &caps["workspaceSymbolProvider"];
        assert!(ws_provider.is_object(), "workspaceSymbolProvider should be an object");
        assert_eq!(ws_provider["resolveProvider"], json!(true), "Should support resolve");
    }
}

#[test]
fn test_workspace_symbol_resolve_unknown_symbol() {
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

    // Try to resolve a symbol without opening the document
    let unknown_symbol = json!({
        "name": "unknown_function",
        "kind": 12,
        "location": {
            "uri": "file:///unknown.pl",
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 10 }
            }
        }
    });

    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/symbol/resolve".into(),
        params: Some(unknown_symbol.clone()),
    };

    let response = server.handle_request(request).unwrap();
    let resolved = response.result.unwrap();
    
    // Should return the original symbol unchanged
    assert_eq!(resolved["name"], "unknown_function");
    assert_eq!(resolved["kind"], 12);
}