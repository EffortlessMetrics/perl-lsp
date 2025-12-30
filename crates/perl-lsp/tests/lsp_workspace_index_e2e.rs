#![allow(clippy::collapsible_if)]

use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use perl_parser::workspace_index::WorkspaceIndex;
use serde_json::json;
use url::Url;

/// Test that workspace index properly tracks cross-file symbols
#[test]
fn test_workspace_index_cross_file() {
    let index = WorkspaceIndex::new();

    // File A: defines Foo::bar
    let uri_a = Url::parse("file:///workspace/A.pm").unwrap();
    let content_a = r#"
package Foo;

sub bar {
    my $x = 42;
    return $x;
}

sub baz {
    return bar() + 1;
}
1;
"#;

    // File B: uses Foo::bar
    let uri_b = Url::parse("file:///workspace/B.pl").unwrap();
    let content_b = r#"
use Foo;

my $result = Foo::bar();
print "Result: $result\n";

Foo::baz();
"#;

    // Index both files
    index.index_file(uri_a.clone(), content_a.to_string()).expect("Should index A.pm");
    index.index_file(uri_b.clone(), content_b.to_string()).expect("Should index B.pl");

    // Check that Foo::bar is defined in A.pm
    let bar_def = index.find_definition("bar");
    assert!(bar_def.is_some(), "Should find definition of 'bar'");
    let def = bar_def.unwrap();
    assert_eq!(def.uri, uri_a.to_string(), "Definition should be in A.pm");

    // Check that references to Foo::bar are found
    let bar_refs = index.find_references("bar");
    assert!(!bar_refs.is_empty(), "Should find references to 'bar'");

    // Check workspace symbols search
    let symbols = index.search_symbols("ba");
    assert!(symbols.len() >= 2, "Should find both 'bar' and 'baz'");

    // Test that removing a file updates the index
    index.remove_file(uri_a.as_str());
    let bar_def_after = index.find_definition("bar");
    assert!(bar_def_after.is_none(), "Definition should be removed after file removal");
}

/// Test that index updates when files change
#[test]
fn test_workspace_index_file_updates() {
    let index = WorkspaceIndex::new();
    let uri = Url::parse("file:///workspace/test.pl").unwrap();

    // Initial content
    let content_v1 = r#"
sub old_name {
    return 1;
}
"#;

    index.index_file(uri.clone(), content_v1.to_string()).expect("Should index v1");

    // Check old_name exists
    let old_def = index.find_definition("old_name");
    assert!(old_def.is_some(), "Should find old_name");

    // Update with new content
    let content_v2 = r#"
sub new_name {
    return 2;
}
"#;

    index.index_file(uri.clone(), content_v2.to_string()).expect("Should index v2");

    // Check old_name is gone and new_name exists
    let old_def_after = index.find_definition("old_name");
    assert!(old_def_after.is_none(), "old_name should be gone");

    let new_def = index.find_definition("new_name");
    assert!(new_def.is_some(), "Should find new_name");
}

/// Test LSP workspace/symbol request with index
#[test]
#[ignore] // Missing `initialized` notification after initialize request - Server not initialized error
fn test_lsp_workspace_symbols_with_index() {
    let mut server = LspServer::new();

    // Initialize server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///workspace",
            "capabilities": {}
        })),
    };
    server.handle_request(init_request);

    // Open two files
    let open_a = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///workspace/A.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package Foo;\nsub bar { return 42; }\n1;"
            }
        })),
    };
    server.handle_request(open_a);

    let open_b = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///workspace/B.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package Bar;\nsub baz { return 'hello'; }\n1;"
            }
        })),
    };
    server.handle_request(open_b);

    // Search for workspace symbols
    let search_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "workspace/symbol".to_string(),
        params: Some(json!({
            "query": "ba"
        })),
    };

    let response = server.handle_request(search_request).unwrap();
    assert!(response.error.is_none(), "Should not return error");

    if let Some(result) = response.result {
        if let Some(symbols) = result.as_array() {
            assert!(symbols.len() >= 2, "Should find at least bar and baz");

            // Check that both symbols are present
            let names: Vec<String> = symbols
                .iter()
                .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                .map(|s| s.to_string())
                .collect();

            assert!(names.contains(&"bar".to_string()), "Should find 'bar'");
            assert!(names.contains(&"baz".to_string()), "Should find 'baz'");
        } else {
            panic!("Result should be an array");
        }
    } else {
        panic!("Should return a result");
    }
}

/// Test cross-file go-to-definition
#[test]
#[ignore] // Missing `initialized` notification after initialize request - Server not initialized error
fn test_cross_file_definition() {
    let mut server = LspServer::new();

    // Initialize
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///workspace",
            "capabilities": {}
        })),
    };
    server.handle_request(init_request);

    // Open file with definition
    let open_def = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///workspace/Lib.pm",
                "languageId": "perl",
                "version": 1,
                "text": "package Lib;\nsub helper {\n    return 'help';\n}\n1;"
            }
        })),
    };
    server.handle_request(open_def);

    // Open file with usage
    let open_use = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///workspace/main.pl",
                "languageId": "perl",
                "version": 1,
                "text": "use Lib;\nmy $x = Lib::helper();"
            }
        })),
    };
    server.handle_request(open_use);

    // Request definition from usage site
    let def_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "textDocument/definition".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///workspace/main.pl"
            },
            "position": {
                "line": 1,
                "character": 15  // Position on "helper"
            }
        })),
    };

    let response = server.handle_request(def_request).unwrap();
    assert!(response.error.is_none(), "Should not return error");

    if let Some(result) = response.result {
        // Definition should point to Lib.pm
        if let Some(location) = result.as_object() {
            if let Some(uri) = location.get("uri").and_then(|u| u.as_str()) {
                assert!(uri.contains("Lib.pm"), "Definition should be in Lib.pm");
            } else if let Some(locations) = result.as_array() {
                // Handle multiple locations
                assert!(!locations.is_empty(), "Should have at least one location");
                if let Some(first) = locations.first() {
                    if let Some(uri) = first.get("uri").and_then(|u| u.as_str()) {
                        assert!(uri.contains("Lib.pm"), "Definition should be in Lib.pm");
                    }
                }
            }
        }
    }
}
