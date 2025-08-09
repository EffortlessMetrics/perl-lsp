use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

/// A simple LSP client for testing the LSP server
pub struct LspClient {
    child: std::process::Child,
}

impl LspClient {
    /// Spawn a new LSP server process
    pub fn spawn(bin: &str) -> Self {
        let child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start LSP server");
        
        let mut client = Self { child };
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

    /// Receive a JSON-RPC message from the server
    fn recv(&mut self) -> Value {
        let stdout = self.child.stdout.as_mut().expect("stdout not available");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        
        // Read headers until we find Content-Length
        let content_length = loop {
            line.clear();
            reader.read_line(&mut line).expect("Failed to read line");
            
            if line.starts_with("Content-Length:") {
                let len_str = line["Content-Length:".len()..].trim();
                let len = len_str.parse::<usize>().expect("Invalid Content-Length");
                
                // Consume the blank line after headers
                let mut blank = [0u8; 2];
                reader.read_exact(&mut blank).expect("Failed to read blank line");
                
                break len;
            }
        };
        
        // Read the JSON body
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).expect("Failed to read body");
        
        serde_json::from_slice(&body).expect("Failed to parse JSON response")
    }

    /// Initialize the LSP connection
    fn initialize(&mut self) {
        // Send initialize request
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {}
            }
        }));
        
        // Receive initialize response
        let _response = self.recv();
        
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

    /// Send a request and receive a response
    pub fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }));
        
        self.recv()
    }

    /// Shutdown the LSP server gracefully
    pub fn shutdown(&mut self) {
        // Send shutdown request
        let _ = self.request(99, "shutdown", serde_json::json!(null));
        
        // Send exit notification
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }));
        
        // Give the server a moment to shut down
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Kill the process if it's still running
        let _ = self.child.kill();
    }
}