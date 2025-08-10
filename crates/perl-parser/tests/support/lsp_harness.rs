//! LSP Test Harness for Real JSON-RPC Testing
//!
//! Provides a test harness that communicates with the LSP server using real JSON-RPC protocol.

use perl_parser::lsp_server::LspServer;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Cursor, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

/// LSP Test Harness for testing with real JSON-RPC protocol
pub struct LspHarness {
    server: LspServer,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    notification_buffer: Arc<Mutex<VecDeque<Value>>>,
    next_request_id: i32,
}

impl LspHarness {
    /// Create a new test harness
    pub fn new() -> Self {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let notification_buffer = Arc::new(Mutex::new(VecDeque::new()));
        
        // Create server with captured output
        let server = LspServer::with_output(Arc::new(Mutex::new(Box::new(TestWriter {
            buffer: output_buffer.clone(),
            notifications: notification_buffer.clone(),
        }) as Box<dyn Write + Send>)));
        
        Self {
            server,
            output_buffer,
            notification_buffer,
            next_request_id: 1,
        }
    }

    /// Initialize the LSP server
    pub fn initialize(&mut self, capabilities: Option<Value>) -> Result<Value, String> {
        let caps = capabilities.unwrap_or_else(|| json!({
            "textDocument": {
                "completion": {
                    "completionItem": {
                        "snippetSupport": true
                    }
                },
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                },
                "signatureHelp": {
                    "signatureInformation": {
                        "documentationFormat": ["markdown", "plaintext"]
                    }
                }
            }
        }));

        let init_request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "capabilities": caps,
                "rootUri": "file:///workspace"
            }
        });
        self.next_request_id += 1;

        let response = self.send_request(init_request)?;
        
        // Send initialized notification
        self.notify("initialized", json!({}));
        
        Ok(response)
    }

    /// Open a document
    pub fn open(&mut self, uri: &str, text: &str) -> Result<(), String> {
        self.notify("textDocument/didOpen", json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        }));
        Ok(())
    }

    /// Send a request and wait for response
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.next_request_id,
            "method": method,
            "params": params
        });
        self.next_request_id += 1;
        
        self.send_request(request)
    }

    /// Send a notification (no response expected)
    pub fn notify(&mut self, method: &str, params: Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        
        let request_str = format!("{}\r\n", notification.to_string());
        let content = format!(
            "Content-Length: {}\r\n\r\n{}",
            request_str.len(),
            request_str
        );
        
        let mut input = Cursor::new(content.into_bytes());
        let _result = self.server.handle_message(&mut input);
    }

    /// Drain notifications from the buffer
    pub fn drain_notifications(&mut self, method: Option<&str>, timeout_ms: u64) -> Vec<Value> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        
        // Wait a bit for notifications to arrive
        while start.elapsed() < timeout {
            std::thread::sleep(Duration::from_millis(10));
            
            let notifications = self.notification_buffer.lock().unwrap();
            if !notifications.is_empty() {
                break;
            }
        }
        
        let mut notifications = self.notification_buffer.lock().unwrap();
        let mut result = Vec::new();
        
        while let Some(notif) = notifications.pop_front() {
            if let Some(filter_method) = method {
                if notif["method"].as_str() == Some(filter_method) {
                    result.push(notif);
                } else {
                    // Put it back if it doesn't match
                    notifications.push_back(notif);
                    break;
                }
            } else {
                result.push(notif);
            }
        }
        
        result
    }

    /// Get performance timing for a request
    pub fn timed_request(&mut self, method: &str, params: Value) -> Result<(Value, Duration), String> {
        let start = Instant::now();
        let result = self.request(method, params)?;
        let duration = start.elapsed();
        Ok((result, duration))
    }

    // Private helper to send request and get response
    fn send_request(&mut self, request: Value) -> Result<Value, String> {
        // Clear output buffer
        self.output_buffer.lock().unwrap().clear();
        
        // Format request with Content-Length header
        let request_str = request.to_string();
        let content = format!(
            "Content-Length: {}\r\n\r\n{}",
            request_str.len(),
            request_str
        );
        
        // Send to server
        let mut input = Cursor::new(content.into_bytes());
        let result = self.server.handle_message(&mut input);
        
        if let Err(e) = result {
            return Err(format!("Server error: {:?}", e));
        }
        
        // Parse response from output buffer
        let output = self.output_buffer.lock().unwrap();
        let output_str = String::from_utf8_lossy(&output);
        
        // Find the JSON response after headers
        if let Some(json_start) = output_str.find("{") {
            let json_str = &output_str[json_start..];
            if let Ok(response) = serde_json::from_str::<Value>(json_str) {
                if let Some(error) = response.get("error") {
                    return Err(format!("LSP error: {:?}", error));
                }
                if let Some(result) = response.get("result") {
                    return Ok(result.clone());
                }
            }
        }
        
        Err("No valid response received".to_string())
    }
}

/// Test writer that captures output
struct TestWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
    notifications: Arc<Mutex<VecDeque<Value>>>,
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);
        
        // Try to parse as notification
        let content = String::from_utf8_lossy(buf);
        if let Some(json_start) = content.find("{") {
            let json_str = &content[json_start..];
            if let Ok(value) = serde_json::from_str::<Value>(json_str) {
                if value.get("method").is_some() && value.get("id").is_none() {
                    // It's a notification
                    self.notifications.lock().unwrap().push_back(value);
                }
            }
        }
        
        Ok(buf.len())
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Convenience macros

/// Macro for setting up a test with an open document
#[macro_export]
macro_rules! with_open_doc {
    ($uri:expr, $text:expr, $harness:ident, $body:block) => {
        {
            let mut $harness = LspHarness::new();
            $harness.initialize(None).expect("Failed to initialize");
            $harness.open($uri, $text).expect("Failed to open document");
            $body
        }
    };
}

/// Macro for asserting response contains expected locations
#[macro_export]
macro_rules! assert_locations {
    ($response:expr, [$( ($uri:expr, ($sl:expr, $sc:expr)..($el:expr, $ec:expr)) ),*]) => {
        {
            let locations = $response.as_array().expect("Response should be array");
            let expected = vec![
                $( (
                    $uri,
                    ($sl, $sc),
                    ($el, $ec)
                ) ),*
            ];
            
            assert_eq!(locations.len(), expected.len(), "Location count mismatch");
            
            for (i, (uri, (sl, sc), (el, ec))) in expected.iter().enumerate() {
                let loc = &locations[i];
                assert_eq!(loc["uri"].as_str(), Some(*uri));
                assert_eq!(loc["range"]["start"]["line"].as_u64(), Some(*sl as u64));
                assert_eq!(loc["range"]["start"]["character"].as_u64(), Some(*sc as u64));
                assert_eq!(loc["range"]["end"]["line"].as_u64(), Some(*el as u64));
                assert_eq!(loc["range"]["end"]["character"].as_u64(), Some(*ec as u64));
            }
        }
    };
}

/// Macro for asserting highlights
#[macro_export]
macro_rules! assert_highlights {
    ($response:expr, [$( (($sl:expr, $sc:expr)..($el:expr, $ec:expr), $kind:expr) ),*]) => {
        {
            let highlights = $response.as_array().expect("Response should be array");
            let expected = vec![
                $( (
                    ($sl, $sc),
                    ($el, $ec),
                    $kind
                ) ),*
            ];
            
            assert_eq!(highlights.len(), expected.len(), "Highlight count mismatch");
            
            for (i, ((sl, sc), (el, ec), kind)) in expected.iter().enumerate() {
                let hl = &highlights[i];
                assert_eq!(hl["range"]["start"]["line"].as_u64(), Some(*sl as u64));
                assert_eq!(hl["range"]["start"]["character"].as_u64(), Some(*sc as u64));
                assert_eq!(hl["range"]["end"]["line"].as_u64(), Some(*el as u64));
                assert_eq!(hl["range"]["end"]["character"].as_u64(), Some(*ec as u64));
                
                let actual_kind = hl["kind"].as_u64().unwrap_or(1);
                let expected_kind = match kind.as_str() {
                    "Read" => 1,
                    "Write" => 2,
                    "Text" => 3,
                    _ => 1,
                };
                assert_eq!(actual_kind, expected_kind as u64, "Highlight kind mismatch");
            }
        }
    };
}

/// Assert no diagnostics were published
#[macro_export]
macro_rules! assert_no_diags {
    ($harness:expr) => {
        {
            let diags = $harness.drain_notifications(Some("textDocument/publishDiagnostics"), 100);
            assert!(diags.is_empty(), "Expected no diagnostics, got: {:?}", diags);
        }
    };
}

/// Assert performance timing
#[macro_export]
macro_rules! assert_perf {
    ($duration:expr, < $max_ms:expr) => {
        {
            let max = std::time::Duration::from_millis($max_ms);
            assert!(
                $duration < max,
                "Performance assertion failed: {:?} >= {:?}ms",
                $duration,
                $max_ms
            );
        }
    };
}