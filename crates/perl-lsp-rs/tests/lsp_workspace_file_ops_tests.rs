//! Tests for workspace file operation handlers

use parking_lot::Mutex;
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
struct OutputCapture {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl OutputCapture {
    fn new() -> Self {
        Self { buffer: Arc::new(Mutex::new(Vec::new())) }
    }

    fn clear(&self) {
        self.buffer.lock().clear();
    }

    fn messages(&self) -> Vec<Value> {
        let buffer = self.buffer.lock();
        let content = String::from_utf8_lossy(&buffer);
        let mut messages = Vec::new();

        for chunk in content.split("\r\n\r\n") {
            if chunk.trim().is_empty() {
                continue;
            }
            if let Some(json_str) = chunk.lines().nth(1) {
                if let Ok(msg) = serde_json::from_str::<Value>(json_str) {
                    messages.push(msg);
                }
            } else if !chunk.starts_with("Content-Length") {
                if let Ok(msg) = serde_json::from_str::<Value>(chunk) {
                    messages.push(msg);
                }
            }
        }

        messages
    }
}

impl Write for OutputCapture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.lock().flush()
    }
}

fn wait_for_method(output: &OutputCapture, method: &str) -> Option<Value> {
    let deadline = Instant::now() + Duration::from_millis(250);
    loop {
        if let Some(message) =
            output.messages().into_iter().find(|message| message["method"].as_str() == Some(method))
        {
            return Some(message);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Helper to create a test LSP server
fn create_test_server() -> LspServer {
    let output = Arc::new(Mutex::new(Box::new(Vec::new()) as Box<dyn std::io::Write + Send>));
    LspServer::with_output(output)
}

fn create_test_server_with_output() -> (LspServer, OutputCapture) {
    let output = OutputCapture::new();
    let server = LspServer::with_output(Arc::new(Mutex::new(
        Box::new(output.clone()) as Box<dyn Write + Send>
    )));
    (server, output)
}

/// Helper to make a request to the server
fn make_request(
    server: &LspServer,
    method: &str,
    params: Option<Value>,
) -> Result<Option<Value>, String> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer((1) as i64)),
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

/// Helper to send the initialized notification (required after initialize request)
fn send_initialized(server: &LspServer) {
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);
}

#[test]
fn test_did_change_watched_files_created() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server first
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Send a file created notification
    let params = json!({
        "changes": [
            {
                "uri": "file:///test/workspace/new_module.pm",
                "type": 1  // Created
            }
        ]
    });

    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

#[test]
fn test_did_change_watched_files_changed() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // First open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

    // Send a file changed notification
    let params = json!({
        "changes": [
            {
                "uri": "file:///test/workspace/test.pl",
                "type": 2  // Changed
            }
        ]
    });

    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

#[test]
fn test_did_change_watched_files_deleted() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // First open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

    // Send a file deleted notification
    let params = json!({
        "changes": [
            {
                "uri": "file:///test/workspace/test.pl",
                "type": 3  // Deleted
            }
        ]
    });

    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

/// Verify that a DELETED event removes the document from the in-memory store.
///
/// Acceptance criterion: "Deleted files are removed from index and symbol cache."
///
/// Uses `test_has_document` which requires the `expose_lsp_test_api` feature.
#[cfg(feature = "expose_lsp_test_api")]
#[test]
fn test_did_change_watched_files_deleted_removes_from_store()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open a document so it lives in the in-memory store.
    let uri = "file:///test/workspace/to_delete.pl";
    let open_params = json!({
        "textDocument": {
            "uri": uri,
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

    assert!(server.test_has_document(uri), "document must be in store after didOpen");

    // Send a DELETED event for that file.
    let params = json!({
        "changes": [{"uri": uri, "type": 3}]  // 3 = Deleted
    });
    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));
    assert!(result.is_ok());
    assert_eq!(result?, None, "notification must return None");

    // The document must have been evicted from the store.
    assert!(
        !server.test_has_document(uri),
        "deleted file must be removed from document store after DELETED event"
    );
    Ok(())
}

/// Verify that non-Perl files (`.log`, `.tmp`) in a didChangeWatchedFiles
/// notification are handled gracefully and do not crash the server.
///
/// Acceptance criterion: "Only Perl source files trigger re-indexing."
#[test]
fn test_did_change_watched_files_non_perl_files_handled_gracefully()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Send non-Perl file change events -- should not crash and should return None.
    let params = json!({
        "changes": [
            {"uri": "file:///test/workspace/debug.log", "type": 2},
            {"uri": "file:///test/workspace/cache.tmp", "type": 1},
            {"uri": "file:///test/workspace/Makefile", "type": 2},
            {"uri": "file:///test/workspace/.gitignore", "type": 1},
        ]
    });
    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    assert!(result.is_ok(), "non-Perl file events must not produce an error");
    assert_eq!(result?, None);

    // Server must remain responsive after receiving non-Perl file events.
    let symbol_result = make_request(&server, "workspace/symbol", Some(json!({"query": ""})));
    assert!(symbol_result.is_ok(), "server must still respond after non-Perl file events");
    Ok(())
}

/// Verify that a batch with multiple changes of different types are all processed
/// without crashing and the notification returns None.
///
/// Acceptance criterion: multiple events in one notification (create + change + delete).
#[test]
fn test_did_change_watched_files_multiple_mixed_events() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open two documents: one that will be changed and one that will be deleted.
    let changed_uri = "file:///test/workspace/changed.pl";
    let deleted_uri = "file:///test/workspace/deleted.pl";

    for uri in &[changed_uri, deleted_uri] {
        let open_params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "use strict;\n1;\n"
            }
        });
        let _ = make_request(&server, "textDocument/didOpen", Some(open_params));
    }

    // Send a mixed batch: create + change + delete in a single notification.
    let params = json!({
        "changes": [
            {"uri": "file:///test/workspace/new_module.pm", "type": 1},
            {"uri": changed_uri, "type": 2},
            {"uri": deleted_uri, "type": 3},
        ]
    });
    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    assert!(result.is_ok(), "mixed-event batch must succeed");
    assert_eq!(result?, None, "notification must return None");

    // Server must still be responsive after processing the batch.
    let symbol_result = make_request(&server, "workspace/symbol", Some(json!({"query": ""})));
    assert!(symbol_result.is_ok(), "server must remain responsive after batch processing");
    Ok(())
}

/// Verify that a batch with multiple changes includes correct behavioral outcome
/// for the DELETED event: the document is removed from the in-memory store.
///
/// Requires the `expose_lsp_test_api` feature for `test_has_document`.
#[cfg(feature = "expose_lsp_test_api")]
#[test]
fn test_did_change_watched_files_mixed_batch_deleted_removed()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    let changed_uri = "file:///test/workspace/changed2.pl";
    let deleted_uri = "file:///test/workspace/deleted2.pl";

    for uri in &[changed_uri, deleted_uri] {
        let open_params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "use strict;\n1;\n"
            }
        });
        let _ = make_request(&server, "textDocument/didOpen", Some(open_params));
    }

    assert!(server.test_has_document(changed_uri));
    assert!(server.test_has_document(deleted_uri));

    let params = json!({
        "changes": [
            {"uri": "file:///test/workspace/new_module2.pm", "type": 1},
            {"uri": changed_uri, "type": 2},
            {"uri": deleted_uri, "type": 3},
        ]
    });
    let _ = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    // The deleted document must have been evicted from the store.
    assert!(!server.test_has_document(deleted_uri), "deleted file must be removed");
    // The changed document must still be present (it was not deleted).
    assert!(server.test_has_document(changed_uri), "changed file must still be present");
    Ok(())
}

/// Verify that an empty changes array is handled gracefully.
#[test]
fn test_did_change_watched_files_empty_changes_array() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    let params = json!({"changes": []});
    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    assert!(result.is_ok(), "empty changes array must not produce an error");
    assert_eq!(result?, None);
    Ok(())
}

/// Verify that a DELETED event for a URI that was never opened is handled
/// gracefully (no panic or error).
#[test]
fn test_did_change_watched_files_delete_unknown_file() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Delete a file that was never opened -- must not crash or error.
    let params = json!({
        "changes": [{"uri": "file:///test/workspace/never_opened.pl", "type": 3}]
    });
    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

#[test]
fn test_did_change_watched_files_invalid_uri() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Send notification with invalid URI (missing uri field)
    let params = json!({
        "changes": [
            {
                "type": 1  // Created, but no URI
            }
        ]
    });

    let result = make_request(&server, "workspace/didChangeWatchedFiles", Some(params));

    // Should handle gracefully
    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

#[test]
fn test_will_rename_files() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open a document that uses a module
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/main.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use lib 'lib';\nuse MyModule;\nuse parent 'MyModule';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

    // Request to rename a module file
    let params = json!({
        "files": [
            {
                "oldUri": "file:///test/workspace/lib/MyModule.pm",
                "newUri": "file:///test/workspace/lib/RenamedModule.pm"
            }
        ]
    });

    let result = make_request(&server, "workspace/willRenameFiles", Some(params));

    // Should return a workspace edit (potentially empty if no references found)
    let edit = result?.ok_or("expected workspace edit response")?;
    assert!(edit.is_object());
    assert!(edit.get("changes").is_some());
    Ok(())
}

#[test]
fn test_will_rename_files_returns_module_import_edits() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open the renamed module and a dependent file that imports it.
    let module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/MyModule.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package MyModule;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(module_open));

    let dependent_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/main.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use MyModule;\nuse parent 'MyModule';\nrequire MyModule;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(dependent_open));

    // Request rename edits for module file rename.
    let params = json!({
        "files": [
            {
                "oldUri": "file:///test/workspace/lib/MyModule.pm",
                "newUri": "file:///test/workspace/lib/RenamedModule.pm"
            }
        ]
    });

    let edit = make_request(&server, "workspace/willRenameFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    let changes =
        edit.get("changes").and_then(Value::as_object).ok_or("expected changes object")?;
    let main_changes = changes
        .get("file:///test/workspace/main.pl")
        .and_then(Value::as_array)
        .ok_or("expected edits for dependent main.pl")?;

    let new_texts: Vec<String> = main_changes
        .iter()
        .filter_map(|entry| entry.get("newText").and_then(Value::as_str).map(ToString::to_string))
        .collect();

    assert!(
        new_texts.contains(&"use RenamedModule;".to_string()),
        "expected rewritten use import in edits: {new_texts:?}"
    );
    assert!(
        new_texts.contains(&"use parent 'RenamedModule';".to_string()),
        "expected rewritten parent import in edits: {new_texts:?}"
    );
    assert!(
        new_texts.contains(&"require RenamedModule;".to_string()),
        "expected rewritten require import in edits: {new_texts:?}"
    );

    Ok(())
}

#[test]
fn test_will_rename_files_coalesces_multi_rename_edits_per_line()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    let first_module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/Foo/Bar.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package Foo::Bar;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(first_module_open));

    let second_module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/Foo/Baz.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package Foo::Baz;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(second_module_open));

    let dependent_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/main.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use Foo::Bar; use Foo::Baz;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(dependent_open));

    let params = json!({
        "files": [
            {
                "oldUri": "file:///test/workspace/lib/Foo/Bar.pm",
                "newUri": "file:///test/workspace/lib/Renamed/Bar.pm"
            },
            {
                "oldUri": "file:///test/workspace/lib/Foo/Baz.pm",
                "newUri": "file:///test/workspace/lib/Renamed/Baz.pm"
            }
        ]
    });

    let edit = make_request(&server, "workspace/willRenameFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    let changes =
        edit.get("changes").and_then(Value::as_object).ok_or("expected changes object")?;
    let main_changes = changes
        .get("file:///test/workspace/main.pl")
        .and_then(Value::as_array)
        .ok_or("expected edits for dependent main.pl")?;

    assert_eq!(
        main_changes.len(),
        1,
        "expected one coalesced edit for the dependent line, got: {main_changes:?}"
    );
    assert_eq!(
        main_changes[0].get("newText").and_then(Value::as_str),
        Some("use Renamed::Bar; use Renamed::Baz;")
    );

    Ok(())
}

#[test]
fn test_will_rename_files_missing_uri() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Request with missing URIs
    let params = json!({
        "files": [
            {
                // Missing oldUri and newUri
            }
        ]
    });

    let result = make_request(&server, "workspace/willRenameFiles", Some(params));

    // Should handle gracefully and return empty edit
    let edit = result?.ok_or("expected workspace edit response")?;
    assert!(edit.is_object());
    assert_eq!(edit.get("changes"), Some(&json!({})));
    Ok(())
}

#[test]
fn test_did_delete_files() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nprint 'Hello';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

    // Send delete notification
    let params = json!({
        "files": [
            {
                "uri": "file:///test/workspace/test.pl"
            }
        ]
    });

    let result = make_request(&server, "workspace/didDeleteFiles", Some(params));

    // This is a notification, should return None
    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

#[test]
fn test_did_delete_files_invalid_uri() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Send delete notification with missing URI
    let params = json!({
        "files": [
            {
                // Missing uri field
            }
        ]
    });

    let result = make_request(&server, "workspace/didDeleteFiles", Some(params));

    // Should handle gracefully
    assert!(result.is_ok());
    assert_eq!(result?, None);
    Ok(())
}

#[test]
fn test_will_delete_files_skips_warnings_for_co_deleted_dependents()
-> Result<(), Box<dyn std::error::Error>> {
    let (server, output) = create_test_server_with_output();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);
    output.clear();

    let module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/MyModule.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package MyModule;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(module_open));

    let dependent_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/main.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use MyModule;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(dependent_open));
    output.clear();

    let params = json!({
        "files": [
            { "uri": "file:///test/workspace/lib/MyModule.pm" },
            { "uri": "file:///test/workspace/main.pl" }
        ]
    });

    let edit = make_request(&server, "workspace/willDeleteFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    assert!(edit.is_object());
    assert!(
        wait_for_method(&output, "window/showMessage").is_none(),
        "co-deleted dependents should not trigger a safe-delete warning"
    );

    Ok(())
}

#[test]
fn test_will_delete_files_aggregates_warning_for_multiple_unsafe_deletes()
-> Result<(), Box<dyn std::error::Error>> {
    let (server, output) = create_test_server_with_output();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);
    output.clear();

    let first_module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/Alpha.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package Alpha;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(first_module_open));

    let second_module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/Beta.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package Beta;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(second_module_open));

    let first_dependent = json!({
        "textDocument": {
            "uri": "file:///test/workspace/app.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use Alpha;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(first_dependent));

    let second_dependent = json!({
        "textDocument": {
            "uri": "file:///test/workspace/bin/tool.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use Beta;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(second_dependent));
    output.clear();

    let params = json!({
        "files": [
            { "uri": "file:///test/workspace/lib/Alpha.pm" },
            { "uri": "file:///test/workspace/lib/Beta.pm" }
        ]
    });

    let edit = make_request(&server, "workspace/willDeleteFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    assert!(edit.is_object());

    let message = wait_for_method(&output, "window/showMessage")
        .ok_or("expected aggregated safe-delete warning notification")?;
    let message_text =
        message["params"]["message"].as_str().ok_or("expected warning message text")?;
    assert!(
        message_text.contains("2 files have dependent workspace files"),
        "expected aggregated safe-delete warning, got: {message_text}"
    );

    Ok(())
}

#[test]
fn test_will_delete_files_warns_for_cross_file_symbol_usage_without_module_import()
-> Result<(), Box<dyn std::error::Error>> {
    let (server, output) = create_test_server_with_output();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);
    output.clear();

    let utility_module = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/Utility.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package Utility;\nsub helper { return 42; }\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(utility_module));

    let consumer_script = json!({
        "textDocument": {
            "uri": "file:///test/workspace/bin/use_helper.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use strict;\nuse warnings;\nprint helper();\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(consumer_script));
    output.clear();

    let params = json!({
        "files": [
            { "uri": "file:///test/workspace/lib/Utility.pm" }
        ]
    });

    let edit = make_request(&server, "workspace/willDeleteFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    assert!(edit.is_object());

    let message = wait_for_method(&output, "window/showMessage")
        .ok_or("expected safe-delete warning notification")?;
    let message_text =
        message["params"]["message"].as_str().ok_or("expected warning message text")?;
    assert!(
        message_text.contains("dependent workspace file"),
        "expected safe-delete warning to mention dependent files, got: {message_text}"
    );

    Ok(())
}

#[test]
fn test_apply_edit_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "print 'Hello';\nprint 'World';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

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

    let result = make_request(&server, "workspace/applyEdit", Some(params));

    // Should return success
    let response = result?.ok_or("expected applyEdit response")?;
    assert_eq!(response.get("applied"), Some(&json!(true)));
    Ok(())
}

#[test]
fn test_apply_edit_multi_line() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open a document
    let open_params = json!({
        "textDocument": {
            "uri": "file:///test/workspace/test.pl",
            "languageId": "perl",
            "version": 1,
            "text": "print 'Hello';\nprint 'World';\nprint 'End';\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(open_params));

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

    let result = make_request(&server, "workspace/applyEdit", Some(params));

    // Should return success
    let response = result?.ok_or("expected applyEdit response")?;
    assert_eq!(response.get("applied"), Some(&json!(true)));
    Ok(())
}

#[test]
fn test_apply_edit_no_document() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

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

    let result = make_request(&server, "workspace/applyEdit", Some(params));

    // Should still return success (edit was "applied" even if document doesn't exist)
    let response = result?.ok_or("expected applyEdit response")?;
    assert_eq!(response.get("applied"), Some(&json!(true)));
    Ok(())
}

#[test]
fn test_apply_edit_invalid_params() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Send invalid params (no edit field)
    let params = json!({});

    let result = make_request(&server, "workspace/applyEdit", Some(params));

    // Should return failure
    let response = result?.ok_or("expected applyEdit response")?;
    assert_eq!(response.get("applied"), Some(&json!(false)));
    assert!(response.get("failureReason").is_some());
    Ok(())
}

#[test]
fn test_path_to_module_name() -> Result<(), Box<dyn std::error::Error>> {
    // Test the path_to_module_name function indirectly through willRenameFiles
    let server = create_test_server();

    // Initialize the server
    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Test various path patterns
    let test_cases = vec![
        ("file:///test/lib/Foo/Bar.pm", "file:///test/lib/Baz/Qux.pm"),
        ("file:///test/workspace/lib/Module.pm", "file:///test/workspace/lib/NewModule.pm"),
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

        let result = make_request(&server, "workspace/willRenameFiles", Some(params));

        // Should always succeed and return a workspace edit
        let edit = result?.ok_or("expected workspace edit response")?;
        assert!(edit.is_object());
        assert!(edit.get("changes").is_some());
    }
    Ok(())
}

/// Regression test for #2747: a file that only has `use parent 'Mod'` (no direct `use Mod`)
/// must be discovered by find_dependents and appear in the willRenameFiles edit response.
#[test]
fn test_will_rename_files_pure_parent_only() -> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open the module that will be renamed.
    let module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/MyBase.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package MyBase;\nsub new { bless {}, shift }\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(module_open));

    // Open a dependent file that ONLY has use parent — no direct `use MyBase`.
    let dependent_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/child.pl",
            "languageId": "perl",
            "version": 1,
            "text": "package Child;\nuse parent 'MyBase';\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(dependent_open));

    let params = json!({
        "files": [{
            "oldUri": "file:///test/workspace/lib/MyBase.pm",
            "newUri": "file:///test/workspace/lib/RenamedBase.pm"
        }]
    });

    let edit = make_request(&server, "workspace/willRenameFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    let changes =
        edit.get("changes").and_then(Value::as_object).ok_or("expected changes object")?;

    // The pure-parent-only file must be in the edit response (regression for #2747).
    let child_changes = changes
        .get("file:///test/workspace/child.pl")
        .and_then(Value::as_array)
        .ok_or("expected edits for child.pl — pure use parent case was not discovered")?;

    let new_texts: Vec<String> = child_changes
        .iter()
        .filter_map(|e| e.get("newText").and_then(Value::as_str).map(ToString::to_string))
        .collect();

    assert!(
        new_texts.contains(&"use parent 'RenamedBase';".to_string()),
        "expected rewritten use parent in edits: {new_texts:?}"
    );
    Ok(())
}

/// Regression: Moo/Moose inheritance/role DSL (`extends`/`with`) should also be
/// discovered as dependency edges and rewritten during workspace/willRenameFiles.
#[test]
fn test_will_rename_files_rewrites_moose_extends_and_with() -> Result<(), Box<dyn std::error::Error>>
{
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    let parent_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/My/App/Parent.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package My::App::Parent;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(parent_open));

    let role_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/My/App/Role.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package My::App::Role;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(role_open));

    let child_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/child.pl",
            "languageId": "perl",
            "version": 1,
            "text": "package Child;\nuse Moo;\nextends 'My::App::Parent';\nwith 'My::App::Role';\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(child_open));

    let parent_rename = json!({
        "files": [{
            "oldUri": "file:///test/workspace/lib/My/App/Parent.pm",
            "newUri": "file:///test/workspace/lib/My/App/RenamedParent.pm"
        }]
    });
    let parent_edit = make_request(&server, "workspace/willRenameFiles", Some(parent_rename))?
        .ok_or("expected workspace edit for parent rename")?;
    let parent_changes = parent_edit
        .get("changes")
        .and_then(Value::as_object)
        .ok_or("expected changes object for parent rename")?;
    let child_changes = parent_changes
        .get("file:///test/workspace/child.pl")
        .and_then(Value::as_array)
        .ok_or("expected edits for child.pl on extends rename")?;
    let parent_new_texts: Vec<String> = child_changes
        .iter()
        .filter_map(|e| e.get("newText").and_then(Value::as_str).map(ToString::to_string))
        .collect();
    assert!(
        parent_new_texts.contains(&"extends 'My::App::RenamedParent';".to_string()),
        "expected rewritten extends in edits: {parent_new_texts:?}"
    );

    let role_rename = json!({
        "files": [{
            "oldUri": "file:///test/workspace/lib/My/App/Role.pm",
            "newUri": "file:///test/workspace/lib/My/App/RenamedRole.pm"
        }]
    });
    let role_edit = make_request(&server, "workspace/willRenameFiles", Some(role_rename))?
        .ok_or("expected workspace edit for role rename")?;
    let role_changes = role_edit
        .get("changes")
        .and_then(Value::as_object)
        .ok_or("expected changes object for role rename")?;
    let child_changes = role_changes
        .get("file:///test/workspace/child.pl")
        .and_then(Value::as_array)
        .ok_or("expected edits for child.pl on with rename")?;
    let role_new_texts: Vec<String> = child_changes
        .iter()
        .filter_map(|e| e.get("newText").and_then(Value::as_str).map(ToString::to_string))
        .collect();
    assert!(
        role_new_texts.contains(&"with 'My::App::RenamedRole';".to_string()),
        "expected rewritten with in edits: {role_new_texts:?}"
    );

    Ok(())
}

/// Regression test: renaming a module whose own file is open should include
/// package declaration edits for the renamed module file itself.
#[test]
fn test_will_rename_files_updates_package_declaration_in_renamed_file()
-> Result<(), Box<dyn std::error::Error>> {
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open ONLY the module file being renamed — no dependent files.
    let module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/Solo.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package Solo;\nsub new { bless {}, shift }\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(module_open));

    let params = json!({
        "files": [{
            "oldUri": "file:///test/workspace/lib/Solo.pm",
            "newUri": "file:///test/workspace/lib/Renamed.pm"
        }]
    });

    let edit = make_request(&server, "workspace/willRenameFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    let changes =
        edit.get("changes").and_then(Value::as_object).ok_or("expected changes object")?;

    let solo_changes = changes
        .get("file:///test/workspace/lib/Solo.pm")
        .and_then(Value::as_array)
        .ok_or("expected package declaration edits for Solo.pm")?;
    let new_texts: Vec<String> = solo_changes
        .iter()
        .filter_map(|e| e.get("newText").and_then(Value::as_str).map(ToString::to_string))
        .collect();

    assert!(
        new_texts.iter().any(|text| text.contains("package Renamed;")),
        "expected package declaration rewrite in Solo.pm, got: {new_texts:?}"
    );
    Ok(())
}

/// Regression test: `workspace/willRenameFiles` must return a valid (possibly
/// empty) response even when the dependent file was opened and then closed before
/// the rename request.  Closing a file removes it from the dependency index so it
/// will not appear in the edit response, but the handler must not panic and must
/// still return edits for files that ARE currently open.
///
/// Design note: in production, files indexed from disk during workspace
/// initialization appear in `find_dependents` but not in `self.documents`.  The
/// three-tier fallback in `read_workspace_text` (open documents → index document
/// store → disk) ensures those files receive edits.  This test validates the
/// open-then-close lifecycle does not break the handler for other open files.
#[test]
fn test_will_rename_files_graceful_with_closed_dependent() -> Result<(), Box<dyn std::error::Error>>
{
    let server = create_test_server();

    let init_params = json!({
        "processId": 1234,
        "rootUri": "file:///test/workspace",
        "capabilities": {}
    });
    let _ = make_request(&server, "initialize", Some(init_params));
    send_initialized(&server);

    // Open the module being renamed so it is indexed.
    let module_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/lib/OldMod.pm",
            "languageId": "perl",
            "version": 1,
            "text": "package OldMod;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(module_open));

    // Open a second consumer that stays open — will receive edits.
    let consumer_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/active.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use OldMod;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(consumer_open));

    // Open an additional dependent and immediately close it.  Closing removes
    // it from both self.documents and the workspace index, so it will not
    // appear in find_dependents — this is expected.  The handler must not crash
    // and must still produce edits for the open files.
    let dep_open = json!({
        "textDocument": {
            "uri": "file:///test/workspace/closed_dep.pl",
            "languageId": "perl",
            "version": 1,
            "text": "use OldMod;\n1;\n"
        }
    });
    let _ = make_request(&server, "textDocument/didOpen", Some(dep_open));

    let dep_close = json!({
        "textDocument": { "uri": "file:///test/workspace/closed_dep.pl" }
    });
    let _ = make_request(&server, "textDocument/didClose", Some(dep_close));

    // Trigger the rename.
    let params = json!({
        "files": [{
            "oldUri": "file:///test/workspace/lib/OldMod.pm",
            "newUri": "file:///test/workspace/lib/NewMod.pm"
        }]
    });

    let edit = make_request(&server, "workspace/willRenameFiles", Some(params))?
        .ok_or("expected workspace edit response")?;
    let changes =
        edit.get("changes").and_then(Value::as_object).ok_or("expected changes object")?;

    // active.pl is still open and must receive edits.
    let active_changes = changes
        .get("file:///test/workspace/active.pl")
        .and_then(Value::as_array)
        .ok_or("expected rename edits for still-open active.pl")?;

    let new_texts: Vec<String> = active_changes
        .iter()
        .filter_map(|e| e.get("newText").and_then(Value::as_str).map(ToString::to_string))
        .collect();

    assert!(
        new_texts.iter().any(|t| t.contains("NewMod")),
        "expected rewritten import in active.pl edits: {new_texts:?}"
    );

    // closed_dep.pl was removed from the index on close, so it is expected to
    // be absent.  We just ensure the handler did not crash.
    Ok(())
}
