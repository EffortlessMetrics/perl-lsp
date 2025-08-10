//! Real tests for Document Lifecycle Events
//! Tests didSave, willSave, and willSaveWaitUntil notifications

use perl_parser::lsp_server::LspServer;
use serde_json::{json, Value};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

mod support;
use support::*;

/// Custom writer that captures output for verification
struct CaptureWriter {
    output: Arc<Mutex<Vec<String>>>,
}

impl CaptureWriter {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let output = Arc::new(Mutex::new(Vec::new()));
        (Self { output: output.clone() }, output)
    }
}

impl std::io::Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).to_string();
        self.output.lock().unwrap().push(msg);
        Ok(buf.len())
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Helper to set up LSP server with document
fn setup_server_with_capture() -> (LspServer, Arc<Mutex<Vec<String>>>, String) {
    let (writer, output) = CaptureWriter::new();
    let mut server = LspServer::new(Box::new(writer));
    
    // Initialize server
    let init_result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {}
        }
    })).unwrap();
    assert!(init_result.is_some());
    
    // Open document
    let uri = "file:///test.pl";
    server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "my $foo = 42;\nuse strict;"
            }
        }
    })).unwrap();
    
    (server, output, uri.to_string())
}

#[test]
fn test_did_save_notification() {
    let (mut server, output, uri) = setup_server_with_capture();
    
    // Clear any existing messages
    output.lock().unwrap().clear();
    
    // Send didSave notification
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": uri,
                "version": 2
            }
        }
    }));
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_none(), "Notifications should not return a response");
    
    // Check that diagnostics were published
    let messages = output.lock().unwrap();
    let has_diagnostics = messages.iter().any(|msg| 
        msg.contains("textDocument/publishDiagnostics")
    );
    assert!(has_diagnostics, "didSave should trigger diagnostics publication");
}

#[test]
fn test_did_save_with_text() {
    let (mut server, output, uri) = setup_server_with_capture();
    
    // Clear any existing messages
    output.lock().unwrap().clear();
    
    // Send didSave notification with text
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": uri,
                "version": 3
            },
            "text": "my $bar = 123;\nuse warnings;"
        }
    }));
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_none(), "Notifications should not return a response");
}

#[test]
fn test_will_save_notification() {
    let (mut server, _output, uri) = setup_server_with_capture();
    
    // Send willSave notification - Manual save
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/willSave",
        "params": {
            "textDocument": {
                "uri": uri.clone()
            },
            "reason": 1 // Manual = 1
        }
    }));
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_none(), "willSave notification should not return a response");
    
    // Test AfterDelay reason
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/willSave",
        "params": {
            "textDocument": {
                "uri": uri.clone()
            },
            "reason": 2 // AfterDelay = 2
        }
    }));
    
    assert!(result.is_ok());
    
    // Test FocusOut reason
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/willSave",
        "params": {
            "textDocument": {
                "uri": uri
            },
            "reason": 3 // FocusOut = 3
        }
    }));
    
    assert!(result.is_ok());
}

#[test]
fn test_will_save_wait_until_no_edits() {
    let (mut server, _output, uri) = setup_server_with_capture();
    
    // Send willSaveWaitUntil request
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "textDocument/willSaveWaitUntil",
        "params": {
            "textDocument": {
                "uri": uri
            },
            "reason": 1 // Manual
        }
    }));
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some(), "willSaveWaitUntil should return a response");
    
    let edits = response.unwrap();
    assert!(edits.is_array(), "Response should be an array of text edits");
    let edits_arr = edits.as_array().unwrap();
    
    // By default, should return empty array (no edits)
    assert_eq!(edits_arr.len(), 0, "Should return empty array when no edits needed");
}

#[test]
fn test_will_save_wait_until_with_formatting() {
    let (mut server, _output, uri) = setup_server_with_capture();
    
    // Update document with poorly formatted code
    server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": uri.clone(),
                "version": 2
            },
            "contentChanges": [{
                "text": "my$foo=42;print$foo;"
            }]
        }
    })).unwrap();
    
    // Send willSaveWaitUntil request
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "textDocument/willSaveWaitUntil",
        "params": {
            "textDocument": {
                "uri": uri
            },
            "reason": 1 // Manual
        }
    }));
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_some(), "willSaveWaitUntil should return a response");
    
    let edits = response.unwrap();
    assert!(edits.is_array(), "Response should be an array of text edits");
    
    // The server may return formatting edits if configured
    // We're just checking the structure is correct
    let edits_arr = edits.as_array().unwrap();
    for edit in edits_arr {
        assert!(edit.is_object());
        let obj = edit.as_object().unwrap();
        assert!(obj.contains_key("range"), "Edit must have range");
        assert!(obj.contains_key("newText"), "Edit must have newText");
        
        let range = &obj["range"];
        assert!(range["start"].is_object(), "Range start must be object");
        assert!(range["end"].is_object(), "Range end must be object");
    }
}

#[test]
fn test_did_close_after_save() {
    let (mut server, output, uri) = setup_server_with_capture();
    
    // Send didSave notification
    server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": uri.clone(),
                "version": 2
            }
        }
    })).unwrap();
    
    // Clear messages
    output.lock().unwrap().clear();
    
    // Send didClose notification
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": {
            "textDocument": {
                "uri": uri
            }
        }
    }));
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.is_none(), "didClose notification should not return a response");
    
    // Check that diagnostics were cleared
    let messages = output.lock().unwrap();
    let has_clear_diagnostics = messages.iter().any(|msg| 
        msg.contains("textDocument/publishDiagnostics") && msg.contains("[]")
    );
    assert!(has_clear_diagnostics, "didClose should clear diagnostics");
}

#[test]
fn test_save_events_sequence() {
    let (mut server, output, uri) = setup_server_with_capture();
    
    // Simulate typical save sequence: willSave -> willSaveWaitUntil -> didSave
    
    // 1. willSave notification
    server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/willSave",
        "params": {
            "textDocument": { "uri": uri.clone() },
            "reason": 1
        }
    })).unwrap();
    
    // 2. willSaveWaitUntil request
    let edits_response = server.handle_request(json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "textDocument/willSaveWaitUntil",
        "params": {
            "textDocument": { "uri": uri.clone() },
            "reason": 1
        }
    })).unwrap();
    
    assert!(edits_response.is_some());
    
    // 3. didSave notification
    server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": uri,
                "version": 3
            }
        }
    })).unwrap();
    
    // Verify diagnostics were published after save
    let messages = output.lock().unwrap();
    let has_diagnostics = messages.iter().any(|msg| 
        msg.contains("textDocument/publishDiagnostics")
    );
    assert!(has_diagnostics, "Full save sequence should trigger diagnostics");
}

#[test]
fn test_save_with_invalid_uri() {
    let (mut server, _output, _uri) = setup_server_with_capture();
    
    // Try to save a document that was never opened
    let result = server.handle_request(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didSave",
        "params": {
            "textDocument": {
                "uri": "file:///nonexistent.pl",
                "version": 1
            }
        }
    }));
    
    // Should handle gracefully without crashing
    assert!(result.is_ok());
}