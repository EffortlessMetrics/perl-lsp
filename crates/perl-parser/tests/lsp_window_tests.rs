//! Tests for window/* and telemetry/event LSP features

use parking_lot::Mutex;
use perl_parser::lsp_server::{LspServer, MessageType, ShowDocumentOptions};
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;

/// Capture output for testing
struct OutputCapture {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl OutputCapture {
    fn new() -> Self {
        Self { buffer: Arc::new(Mutex::new(Vec::new())) }
    }

    fn get_messages(&self) -> Vec<Value> {
        let buffer = self.buffer.lock();
        let content = String::from_utf8_lossy(&buffer);

        let mut messages = Vec::new();
        for chunk in content.split("\r\n\r\n") {
            if chunk.trim().is_empty() {
                continue;
            }
            // Skip Content-Length header
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

    fn clear(&self) {
        self.buffer.lock().clear();
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

impl Clone for OutputCapture {
    fn clone(&self) -> Self {
        Self { buffer: Arc::clone(&self.buffer) }
    }
}

#[test]
fn lsp_window_show_message_request_format() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize server to enable capabilities
    let init_params = json!({
        "capabilities": {
            "window": {
                "showDocument": {
                    "support": true
                },
                "workDoneProgress": true
            }
        }
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    output.clear();

    // Send showMessageRequest
    let result = server.show_message_request(
        MessageType::Warning,
        "Do you want to continue?",
        vec!["Yes", "No"],
    );

    // Verify request was sent
    assert!(result.is_ok());

    let messages = output.get_messages();
    assert!(!messages.is_empty(), "Expected showMessageRequest to be sent");

    let request = &messages[0];
    assert_eq!(request["jsonrpc"], "2.0");
    assert_eq!(request["method"], "window/showMessageRequest");
    assert_eq!(request["params"]["type"], 2); // Warning = 2
    assert_eq!(request["params"]["message"], "Do you want to continue?");

    let actions = request["params"]["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0]["title"], "Yes");
    assert_eq!(actions[1]["title"], "No");
}

#[test]
fn lsp_window_show_document_requires_capability() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Try to show document without capability
    let result =
        server.show_document("file:///test.pl", ShowDocumentOptions { ..Default::default() });

    // Should fail with Unsupported error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    assert!(err.to_string().contains("doesn't support"));
}

#[test]
fn lsp_window_show_document_with_capability() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize with showDocument capability
    let init_params = json!({
        "capabilities": {
            "window": {
                "showDocument": {
                    "support": true
                }
            }
        }
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    output.clear();

    // Send showDocument with options
    let options = ShowDocumentOptions {
        external: false,
        take_focus: true,
        selection: Some(lsp_types::Range {
            start: lsp_types::Position { line: 10, character: 5 },
            end: lsp_types::Position { line: 10, character: 15 },
        }),
    };

    let result = server.show_document("file:///test.pl", options);
    assert!(result.is_ok());

    let messages = output.get_messages();
    assert!(!messages.is_empty());

    let request = &messages[0];
    assert_eq!(request["method"], "window/showDocument");
    assert_eq!(request["params"]["uri"], "file:///test.pl");
    assert_eq!(request["params"]["takeFocus"], true);
    assert!(request["params"]["selection"].is_object());
}

#[test]
fn lsp_window_progress_lifecycle() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize with workDoneProgress capability
    let init_params = json!({
        "capabilities": {
            "window": {
                "workDoneProgress": true
            }
        }
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    output.clear();

    // Create progress token
    let token = "test-progress-1";
    let result = server.create_work_done_progress(token);
    assert!(result.is_ok(), "Failed to create progress: {:?}", result);

    let messages = output.get_messages();
    assert!(!messages.is_empty());
    assert_eq!(messages[0]["method"], "window/workDoneProgress/create");
    assert_eq!(messages[0]["params"]["token"], token);

    output.clear();

    // Report progress begin
    let result = server.report_progress_begin(token, "Indexing", Some("Starting..."));
    assert!(result.is_ok());

    let messages = output.get_messages();
    assert!(!messages.is_empty());
    assert_eq!(messages[0]["method"], "$/progress");
    assert_eq!(messages[0]["params"]["token"], token);
    assert_eq!(messages[0]["params"]["value"]["kind"], "begin");
    assert_eq!(messages[0]["params"]["value"]["title"], "Indexing");
    assert_eq!(messages[0]["params"]["value"]["message"], "Starting...");

    output.clear();

    // Report progress update
    let result = server.report_progress_report(token, Some("50% complete"), Some(50));
    assert!(result.is_ok());

    let messages = output.get_messages();
    assert!(!messages.is_empty());
    assert_eq!(messages[0]["method"], "$/progress");
    assert_eq!(messages[0]["params"]["value"]["kind"], "report");
    assert_eq!(messages[0]["params"]["value"]["percentage"], 50);

    output.clear();

    // Report progress end
    let result = server.report_progress_end(token, Some("Complete"));
    assert!(result.is_ok());

    let messages = output.get_messages();
    assert!(!messages.is_empty());
    assert_eq!(messages[0]["method"], "$/progress");
    assert_eq!(messages[0]["params"]["value"]["kind"], "end");
    assert_eq!(messages[0]["params"]["value"]["message"], "Complete");
}

#[test]
fn lsp_window_progress_duplicate_token_fails() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize with workDoneProgress capability
    let init_params = json!({
        "capabilities": {
            "window": {
                "workDoneProgress": true
            }
        }
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    // Create first token
    let token = "duplicate-token";
    let result = server.create_work_done_progress(token);
    assert!(result.is_ok());

    // Try to create same token again
    let result = server.create_work_done_progress(token);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
}

#[test]
fn lsp_window_progress_cancel_handler() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize with workDoneProgress capability
    let init_params = json!({
        "capabilities": {
            "window": {
                "workDoneProgress": true
            }
        }
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    // Send initialized notification
    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Create progress token
    let token = "cancellable-progress";
    let result = server.create_work_done_progress(token);
    assert!(result.is_ok());

    // Send cancel notification
    let cancel_response = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None, // Notification
        method: "window/workDoneProgress/cancel".to_string(),
        params: Some(json!({ "token": token })),
    });

    // Notification returns None (no response for notifications)
    assert!(cancel_response.is_none());
}

#[test]
fn lsp_window_telemetry_respects_config() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize server first
    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({})),
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    output.clear();

    // Telemetry is disabled by default
    let event = json!({
        "event": "test",
        "data": { "value": 123 }
    });

    let result = server.send_telemetry(event.clone());
    assert!(result.is_ok());

    // No telemetry should be sent (disabled)
    let messages = output.get_messages();
    assert!(messages.is_empty(), "Telemetry sent when disabled");

    output.clear();

    // Enable telemetry via configuration using didChangeConfiguration
    let config_response = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None, // Notification
        method: "workspace/didChangeConfiguration".to_string(),
        params: Some(json!({
            "settings": {
                "perl": {
                    "telemetry": {
                        "enabled": true
                    }
                }
            }
        })),
    });
    // Notification returns None
    assert!(config_response.is_none());

    let result = server.send_telemetry(event);
    assert!(result.is_ok());

    // Telemetry should now be sent
    let messages = output.get_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["method"], "telemetry/event");
    assert_eq!(messages[0]["params"]["event"], "test");
}

#[test]
fn lsp_window_message_types() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Test all message types
    let types = [
        (MessageType::Error, 1),
        (MessageType::Warning, 2),
        (MessageType::Info, 3),
        (MessageType::Log, 4),
    ];

    for (msg_type, expected_value) in types {
        output.clear();

        let _ = server.show_message_request(msg_type, "Test message", vec![]);

        let messages = output.get_messages();
        assert!(!messages.is_empty());
        assert_eq!(messages[0]["params"]["type"], expected_value);
    }
}

#[test]
fn lsp_window_progress_without_capability() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Try to create progress without capability
    let result = server.create_work_done_progress("test");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    assert!(err.to_string().contains("doesn't support"));
}

#[test]
fn lsp_window_show_document_external_flag() {
    let output = OutputCapture::new();
    let output_box: Box<dyn Write + Send> = Box::new(output.clone());
    let mut server = LspServer::with_output(Arc::new(Mutex::new(output_box)));

    // Initialize with showDocument capability
    let init_params = json!({
        "capabilities": {
            "window": {
                "showDocument": {
                    "support": true
                }
            }
        }
    });

    let _ = server.handle_request(perl_parser::lsp_server::JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    output.clear();

    // Test external = true
    let options = ShowDocumentOptions { external: true, take_focus: false, selection: None };

    let _ = server.show_document("https://example.com", options);

    let messages = output.get_messages();
    assert!(!messages.is_empty());
    assert_eq!(messages[0]["params"]["external"], true);
    assert_eq!(messages[0]["params"]["uri"], "https://example.com");
}
