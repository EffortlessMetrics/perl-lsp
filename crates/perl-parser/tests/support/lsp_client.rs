use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A simple LSP client for testing the LSP server
#[allow(dead_code)]
pub struct LspClient {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
    buf: Vec<Value>,  // pending messages (notifications, etc)
    next_id: u64,
}

impl LspClient {
    /// Spawn a new LSP server process
    pub fn spawn(bin: &str) -> Self {
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start LSP server");
        
        let reader = BufReader::new(child.stdout.take().expect("Failed to get stdout"));
        
        let mut client = Self { 
            child,
            reader,
            buf: Vec::new(),
            next_id: 1,
        };
        
        client.initialize();
        client
    }

    /// Send a JSON-RPC message to the server
    fn send(&mut self, message: &Value) {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        
        let stdin = self.child.stdin.as_mut().expect("stdin not available");
        stdin.write_all(header.as_bytes()).expect("Failed to write header");
        stdin.write_all(content.as_bytes()).expect("Failed to write content");
        stdin.flush().expect("Failed to flush stdin");
    }

    /// Receive one message from the server
    fn recv_one(&mut self, timeout_ms: u64) -> Value {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        
        // Read headers
        let mut line = String::new();
        let mut content_length: Option<usize> = None;
        
        loop {
            line.clear();
            if self.reader.read_line(&mut line).expect("Failed to read line") == 0 {
                panic!("LSP server closed stdout unexpectedly");
            }
            
            if line.starts_with("Content-Length:") {
                content_length = Some(
                    line["Content-Length:".len()..]
                        .trim()
                        .parse()
                        .expect("Invalid content length")
                );
            } else if line == "\r\n" {
                break;
            }
            
            if Instant::now() > deadline {
                panic!("Timeout waiting for LSP response headers");
            }
        }
        
        let length = content_length.expect("Missing Content-Length header");
        
        // Read the message body
        let mut body = vec![0u8; length];
        self.reader.read_exact(&mut body).expect("Failed to read body");
        
        serde_json::from_slice(&body).expect("Failed to parse JSON")
    }
    
    /// Receive messages until we get one with the specified id
    fn recv_until_id(&mut self, id: u64) -> Value {
        let timeout_ms = 10_000; // 10 second timeout
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        
        loop {
            // First check buffered messages
            if let Some(pos) = self.buf.iter().position(|m| {
                m.get("id") == Some(&serde_json::json!(id))
            }) {
                return self.buf.remove(pos);
            }
            
            if Instant::now() > deadline {
                panic!("Timeout waiting for response with id {}", id);
            }
            
            // Read a new message
            let msg = self.recv_one(1000);
            
            // Check if this is our response
            if msg.get("id") == Some(&serde_json::json!(id)) {
                return msg;
            }
            
            // Otherwise buffer it (probably a notification)
            self.buf.push(msg);
        }
    }

    /// Initialize the LSP connection
    fn initialize(&mut self) {
        // Send initialize request
        let response = self.request("initialize", serde_json::json!({
            "capabilities": {}
        }));
        
        // Verify initialization succeeded
        if response.get("error").is_some() {
            panic!("Failed to initialize LSP server: {:?}", response);
        }
        
        // Send initialized notification
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }));
    }

    /// Open a document in the LSP server
    pub fn did_open(&mut self, uri: &str, language_id: &str, text: &str) {
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text
                }
            }
        }));
    }

    /// Send a request and wait for response
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        
        self.recv_until_id(id)
    }

    /// Shutdown the LSP server gracefully
    pub fn shutdown(mut self) {
        // Send shutdown request
        let _ = self.request("shutdown", serde_json::json!(null));
        
        // Send exit notification
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }));
        
        // Wait for the process to exit
        let _ = self.child.wait();
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Try to gracefully shutdown
        let _ = self.request("shutdown", serde_json::json!(null));
        let _ = self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }));
        
        // Force kill if still running
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}