use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test Type Hierarchy support (LSP 3.17)
#[test]

fn test_type_hierarchy_prepare() {
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

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized);

    // Open a document with class hierarchy
    let uri = "file:///test.pl";
    let content = r#"package Base;

package Derived;
use parent 'Base';

package Another;
our @ISA = ('Base');

sub new {
    my $class = shift;
    bless {}, $class;
}
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

    // Request type hierarchy at "Base" position (line 0, char 8)
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/prepareTypeHierarchy".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 8 }
        })),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.as_ref().and_then(|r| r.as_array());

    assert!(result.is_some(), "Should return type hierarchy items");
    let items = result.unwrap();
    assert!(!items.is_empty(), "Should have at least one item");

    let item = &items[0];
    assert_eq!(item["name"], "Base");
    assert_eq!(item["kind"], 5); // Class
    assert!(item["uri"].as_str().is_some());
    assert!(item["range"].is_object());
    assert!(item["selectionRange"].is_object());
}

#[test]

fn test_type_hierarchy_supertypes() {
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

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized);

    // Open a document with inheritance
    let uri = "file:///test.pl";
    let content = r#"package Child;
use parent 'Parent1';
use parent 'Parent2';

package Parent1;
package Parent2;
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

    // First prepare type hierarchy on Child
    let prepare_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/prepareTypeHierarchy".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 8 }
        })),
    };

    let prepare_response = server.handle_request(prepare_request).unwrap();
    let items = prepare_response.result.unwrap().as_array().unwrap().clone();
    let child_item = &items[0];

    // Request supertypes
    let supertypes_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "typeHierarchy/supertypes".into(),
        params: Some(json!({
            "item": {
                "name": child_item["name"],
                "kind": child_item["kind"],
                "uri": child_item["uri"],
                "range": child_item["range"],
                "selectionRange": child_item["selectionRange"],
                "data": {
                    "uri": uri,
                    "name": "Child"
                }
            }
        })),
    };

    let response = server.handle_request(supertypes_request).unwrap();
    let result = response.result.as_ref().and_then(|r| r.as_array());

    assert!(result.is_some(), "Should return supertypes");
    let supertypes = result.unwrap();

    // Should find Parent1 and Parent2
    let names: Vec<String> =
        supertypes.iter().filter_map(|item| item["name"].as_str()).map(|s| s.to_string()).collect();

    assert!(names.contains(&"Parent1".to_string()), "Should find Parent1");
    assert!(names.contains(&"Parent2".to_string()), "Should find Parent2");
}

#[test]

fn test_type_hierarchy_subtypes() {
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

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized);

    // Open a document with derived classes
    let uri = "file:///test.pl";
    let content = r#"package Base;

package Derived1;
use parent 'Base';

package Derived2;
our @ISA = ('Base');
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

    // First prepare type hierarchy on Base
    let prepare_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/prepareTypeHierarchy".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 8 }
        })),
    };

    let prepare_response = server.handle_request(prepare_request).unwrap();
    let items = prepare_response.result.unwrap().as_array().unwrap().clone();
    let base_item = &items[0];

    // Request subtypes
    let subtypes_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "typeHierarchy/subtypes".into(),
        params: Some(json!({
            "item": {
                "name": base_item["name"],
                "kind": base_item["kind"],
                "uri": base_item["uri"],
                "range": base_item["range"],
                "selectionRange": base_item["selectionRange"],
                "data": {
                    "uri": uri,
                    "name": "Base"
                }
            }
        })),
    };

    let response = server.handle_request(subtypes_request).unwrap();
    let result = response.result.as_ref().and_then(|r| r.as_array());

    assert!(result.is_some(), "Should return subtypes");
    let subtypes = result.unwrap();

    // Should find Derived1 and Derived2
    let names: Vec<String> =
        subtypes.iter().filter_map(|item| item["name"].as_str()).map(|s| s.to_string()).collect();

    assert!(names.contains(&"Derived1".to_string()), "Should find Derived1");
    assert!(names.contains(&"Derived2".to_string()), "Should find Derived2");
}

#[test]

fn test_type_hierarchy_capability_advertised() {
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

    // Type hierarchy should be advertised in non-lock mode
    if !cfg!(feature = "lsp-ga-lock") {
        assert_eq!(
            caps["typeHierarchyProvider"],
            json!(true),
            "typeHierarchyProvider should be advertised"
        );
    }
}

#[test]

fn test_type_hierarchy_with_namespace_packages() {
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

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized);

    // Test with Foo::Bar -> Foo hierarchy
    let uri = "file:///test.pl";
    let content = r#"package Foo;

package Foo::Bar;

package Foo::Bar::Baz;
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

    // Request type hierarchy at "Foo::Bar"
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/prepareTypeHierarchy".into(),
        params: Some(json!({
            "textDocument": { "uri": uri },
            "position": { "line": 2, "character": 12 }
        })),
    };

    let response = server.handle_request(request).unwrap();
    let result = response.result.as_ref().and_then(|r| r.as_array());

    assert!(result.is_some(), "Should return type hierarchy items");
    let items = result.unwrap();
    assert!(!items.is_empty());

    let item = &items[0];
    assert_eq!(item["name"], "Foo::Bar");
}
