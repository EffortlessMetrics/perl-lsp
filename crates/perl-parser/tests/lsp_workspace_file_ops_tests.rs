//! Tests for workspace file operation handlers

use perl_parser::lsp_server::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::Mutex;

/// Helper to create a test LSP server
fn create_test_server() -> LspServer {
    let output = Arc::new(Mutex::new(
        Box::new(Vec::new()) as Box<dyn std::io::Write + Send>
    ));
    LspServer::with_output(output)
}

/// Helper to make a request to the server
fn make_request(
    server: &mut LspServer,
    method: &str,
    params: Option<Value>,
) -> Result<Option<Value>, String> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: method.to_string(),
        params,
    };

    match server.handle_request(request) {
        Some(response) => {
            if let Some(error) = response.error {
                Err(format!("{}: {}", error.code, error.message))
            } else {
                Ok(response.result)
            }
        }
        None => Ok(None),
    }
}

#[test]
fn test_did_change_watched_files_created() {
    let mut server = create_test_server();

    // Initialize the server first
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Send a file created notification
    let params = json!({
        "changes": [
            {
                "uri": "file:///test/workspace/new_module.pm",
                "type": 1  // Created
            }
        ]
    });

    let result = make_request(&mut server, "workspace/didChangeWatchedFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_did_change_watched_files_changed() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // First open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&mut server, "textDocument/didOpen", Some(open_params));

    // Send a file changed notification
    let params = json!({
        "changes": [
            {
                "uri": "file:///test/workspace/test.pl",
                "type": 2  // Changed
            }
        ]
    });

    let result = make_request(&mut server, "workspace/didChangeWatchedFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_did_change_watched_files_deleted() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // First open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&mut server, "textDocument/didOpen", Some(open_params));

    // Send a file deleted notification
    let params = json!({
        "changes": [
            {
                "uri": "file:///test/workspace/test.pl",
                "type": 3  // Deleted
            }
        ]
    });

    let result = make_request(&mut server, "workspace/didChangeWatchedFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_did_change_watched_files_invalid_uri() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Send notification with invalid URI (missing uri field)
    let params = json!({
        "changes": [
            {
                "type": 1  // Created, but no URI
            }
        ]
    });

    let result = make_request(&mut server, "workspace/didChangeWatchedFiles", Some(params));

    // Should handle gracefully
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_will_rename_files() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Open a document that uses a module
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/main.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use lib 'lib';\nuse MyModule;\nuse parent 'MyModule';\n"
        }
    });
    let _ = make_request(&mut server, "textDocument/didOpen", Some(open_params));

    // Request to rename a module file
    let params = json!({
        "files": [
            {
                "oldUri": "file:///test/workspace/lib/MyModule.pm",
                "newUri": "file:///test/workspace/lib/RenamedModule.pm"
            }
        ]
    });

    let result = make_request(&mut server, "workspace/willRenameFiles", Some(params));

    // Should return a workspace edit (potentially empty if no references found)
    assert!(result.is_ok());
    let edit = result.unwrap().unwrap();
    assert!(edit.is_object());
    assert!(edit.get("changes").is_some());
}

#[test]
fn test_will_rename_files_missing_uri() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Request with missing URIs
    let params = json!({
        "files": [
            {
                // Missing oldUri and newUri
            }
        ]
    });

    let result = make_request(&mut server, "workspace/willRenameFiles", Some(params));

    // Should handle gracefully and return empty edit
    assert!(result.is_ok());
    let edit = result.unwrap().unwrap();
    assert!(edit.is_object());
    assert_eq!(edit.get("changes"), Some(&json!({})));
}

#[test]
fn test_did_delete_files() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&mut server, "textDocument/didOpen", Some(open_params));

    // Send delete notification
    let params = json!({
        "files": [
            {
                "uri": "file:///test/workspace/test.pl"
            }
        ]
    });

    let result = make_request(&mut server, "workspace/didDeleteFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_did_delete_files_invalid_uri() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Send delete notification with missing URI
    let params = json!({
        "files": [
            {
                // Missing uri field
            }
        ]
    });

    let result = make_request(&mut server, "workspace/didDeleteFiles", Some(params));

    // Should handle gracefully
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

#[test]
fn test_apply_edit_single_line() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "print 'Hello';\nprint 'World';\n"
        }
    });
    let _ = make_request(&mut server, "textDocument/didOpen", Some(open_params));

    // Apply an edit
    let params = json!({
        "edit": {
            "changes": {
                "file:///test/workspace/test.pl": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 6},
                            "end": {"line": 0, "character": 13}
                        },
                        "newText": "\"Modified\""
                    }
                ]
            }
        }
    });

    let result = make_request(&mut server, "workspace/applyEdit", Some(params));

    // Should return success
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.get("applied"), Some(&json!(true)));
}

#[test]
fn test_apply_edit_multi_line() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "print 'Hello';\nprint 'World';\nprint 'End';\n"
        }
    });
    let _ = make_request(&mut server, "textDocument/didOpen", Some(open_params));

    // Apply a multi-line edit
    let params = json!({
        "edit": {
            "changes": {
                "file:///test/workspace/test.pl": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 1, "character": 14}
                        },
                        "newText": "# Combined print\nprint 'Hello World';"
                    }
                ]
            }
        }
    });

    let result = make_request(&mut server, "workspace/applyEdit", Some(params));

    // Should return success
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.get("applied"), Some(&json!(true)));
}

#[test]
fn test_apply_edit_no_document() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Try to apply edit to non-existent document
    let params = json!({
        "edit": {
            "changes": {
                "file:///test/workspace/nonexistent.pl": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0}
                        },
                        "newText": "new text"
                    }
                ]
            }
        }
    });

    let result = make_request(&mut server, "workspace/applyEdit", Some(params));

    // Should still return success (edit was "applied" even if document doesn't exist)
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.get("applied"), Some(&json!(true)));
}

#[test]
fn test_apply_edit_invalid_params() {
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Send invalid params (no edit field)
    let params = json!({});

    let result = make_request(&mut server, "workspace/applyEdit", Some(params));

    // Should return failure
    assert!(result.is_ok());
    let response = result.unwrap().unwrap();
    assert_eq!(response.get("applied"), Some(&json!(false)));
    assert!(response.get("failureReason").is_some());
}

#[test]
fn test_path_to_module_name() {
    // Test the path_to_module_name function indirectly through willRenameFiles
    let mut server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&mut server, "initialize", Some(init_params));

    // Test various path patterns
    let test_cases = vec![
        ("file:///test/lib/Foo/Bar.pm", "file:///test/lib/Baz/Qux.pm"),
        (
            "file:///test/workspace/lib/Module.pm",
            "file:///test/workspace/lib/NewModule.pm",
        ),
        ("file:///test/MyModule.pl", "file:///test/YourModule.pl"),
    ];

    for (old_uri, new_uri) in test_cases {
        let params = json!({
            "files": [
                {
                    "oldUri": old_uri,
                    "newUri": new_uri
                }
            ]
        });

        let result = make_request(&mut server, "workspace/willRenameFiles", Some(params));

        // Should always succeed and return a workspace edit
        assert!(result.is_ok());
        let edit = result.unwrap().unwrap();
        assert!(edit.is_object());
        assert!(edit.get("changes").is_some());
    }
}
