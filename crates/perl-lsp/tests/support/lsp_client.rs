#![allow(dead_code)]

use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A simple LSP client for testing the LSP server
#[allow(dead_code)]
pub struct LspClient {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
    buf: Vec<Value>, // pending messages (notifications, etc)
    next_id: u64,
}

impl LspClient {
    /// Spawn a new LSP server process
    pub fn spawn(bin: &str) -> Self {
        Self::spawn_with_env(bin, &[])
    }

    /// Spawn a new LSP server process with environment variables
    pub fn spawn_with_env(bin: &str, env_vars: &[(&str, &str)]) -> Self {
        let mut cmd = Command::new(bin);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());

        // Add any environment variables
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => panic!("Failed to start LSP server '{bin}': {e}"),
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => panic!("Failed to get stdout from LSP server process"),
        };
        let reader = BufReader::new(stdout);

        let mut client = Self { child, reader, buf: Vec::new(), next_id: 1 };

        client.initialize();
        client
    }

    /// Send a JSON-RPC message to the server
    fn send(&mut self, message: &Value) {
        let content = message.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        // Don't panic if stdin is not available (e.g., during drop after error)
        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = stdin.write_all(header.as_bytes());
            let _ = stdin.write_all(content.as_bytes());
            let _ = stdin.flush();
        }
    }

    /// Receive one message from the server
    fn recv_one(&mut self, timeout_ms: u64) -> Value {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        // Read headers (case-insensitive, handle extra headers)
        let mut headers = HashMap::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = match self.reader.read_line(&mut line) {
                Ok(n) => n,
                Err(e) => panic!("Failed to read line from LSP server: {e}"),
            };

            if bytes_read == 0 {
                panic!("LSP server closed stdout unexpectedly");
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                break; // End of headers
            }

            if let Some((key, value)) = line_trimmed.split_once(':') {
                let key = key.trim().to_lowercase();
                headers.insert(key, value.trim().to_string());
            }

            if Instant::now() > deadline {
                panic!("Timeout waiting for LSP response headers");
            }
        }

        // Get content length (case-insensitive)
        let content_length: usize = match headers.get("content-length").and_then(|s| s.parse().ok())
        {
            Some(len) => len,
            None => panic!("Missing or invalid Content-Length header in LSP response"),
        };

        // Read the message body
        let mut body = vec![0u8; content_length];
        if let Err(e) = self.reader.read_exact(&mut body) {
            panic!("Failed to read {content_length} bytes from LSP server: {e}");
        }

        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                let preview = String::from_utf8_lossy(&body);
                panic!("Failed to parse JSON from LSP server: {e}\nBody: {preview}");
            }
        }
    }

    /// Receive messages until we get one with the specified id
    fn recv_until_id(&mut self, id: u64) -> Value {
        let timeout_ms = 10_000; // 10 second timeout
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            // First check buffered messages
            if let Some(pos) =
                self.buf.iter().position(|m| m.get("id") == Some(&serde_json::json!(id)))
            {
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
        // Send initialize request with explicit UTF-16 position encoding
        let response = self.request(
            "initialize",
            json!({
                "capabilities": {
                    "general": {
                        "positionEncodings": ["utf-16"]
                    },
                    "textDocument": {
                        "hover": {
                            "contentFormat": ["markdown", "plaintext"]
                        }
                    }
                }
            }),
        );

        // Verify initialization succeeded
        if response.get("error").is_some() {
            panic!("Failed to initialize LSP server: {:?}", response);
        }

        // Send initialized notification
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }));
    }

    /// Open a document in the LSP server
    pub fn did_open(&mut self, uri: &str, language_id: &str, text: &str) {
        self.send(&json!({
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

        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));

        self.recv_until_id(id)
    }

    /// Shutdown the LSP server gracefully
    pub fn shutdown(mut self) {
        // Send shutdown request (don't wait for response as server may already be closing)
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": "shutdown",
            "params": null
        }));

        // Send exit notification
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }));

        // Wait for the process to exit
        let _ = self.child.wait();

        // Prevent Drop from being called since we already shut down
        std::mem::forget(self);
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Try to gracefully shutdown (but don't wait for response)
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": "shutdown",
            "params": null
        }));
        self.send(&json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }));

        // Force kill if still running
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
