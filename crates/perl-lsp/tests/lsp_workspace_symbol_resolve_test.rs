use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Helper to properly initialize a server with the required initialized notification
fn setup_server() -> LspServer {
    let mut server = LspServer::new();

    // Initialize request
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    let _ = server.handle_request(init_request);

    // Must send initialized notification after initialize
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized);

    server
}

/// Test Workspace Symbol Resolve support (LSP 3.17)
#[test]
fn test_workspace_symbol_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server();

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

    let response = server.handle_request(request).ok_or("handle_request failed")?;
    let resolved = response.result.ok_or("No result in response")?;

    // Check that the symbol was enhanced
    assert_eq!(resolved["name"], "hello");
    assert!(resolved["detail"].is_string(), "Should have detail field");
    let detail_str = resolved["detail"].as_str().ok_or("detail is not a string")?;
    assert!(detail_str.contains("sub"), "Detail should indicate it's a subroutine");

    // Location should still be present
    assert!(resolved["location"].is_object());
    assert!(resolved["location"]["uri"].is_string());
    assert!(resolved["location"]["range"].is_object());

    Ok(())
}

#[test]
fn test_workspace_symbol_resolve_with_container() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server();

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

    let response = server.handle_request(request).ok_or("handle_request failed")?;
    let resolved = response.result.ok_or("No result in response")?;

    // Check enhanced fields
    assert_eq!(resolved["name"], "method_in_package");
    assert!(resolved["detail"].is_string());

    // Should potentially have container information
    // (depending on implementation details)
    if resolved["containerName"].is_string() {
        let container_str =
            resolved["containerName"].as_str().ok_or("containerName is not a string")?;
        assert!(!container_str.is_empty());
    }

    Ok(())
}

#[test]
fn test_workspace_symbol_resolve_capability() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = LspServer::new();

    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };

    let response = server.handle_request(init_request).ok_or("handle_request failed")?;
    let result = response.result.ok_or("No result in response")?;
    let caps = &result["capabilities"];

    // Workspace symbol provider should advertise resolve support in non-lock mode
    if !cfg!(feature = "lsp-ga-lock") {
        let ws_provider = &caps["workspaceSymbolProvider"];
        assert!(ws_provider.is_object(), "workspaceSymbolProvider should be an object");
        assert_eq!(ws_provider["resolveProvider"], json!(true), "Should support resolve");
    }

    Ok(())
}

#[test]
fn test_workspace_symbol_resolve_unknown_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = setup_server();

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

    let response = server.handle_request(request).ok_or("handle_request failed")?;
    let resolved = response.result.ok_or("No result in response")?;

    // Should return the original symbol unchanged
    assert_eq!(resolved["name"], "unknown_function");
    assert_eq!(resolved["kind"], 12);

    Ok(())
}
