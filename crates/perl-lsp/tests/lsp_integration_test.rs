//! Integration tests for the LSP server

use perl_lsp::LspServer;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc;
use std::time::Duration;

/// Helper to create LSP messages
fn create_lsp_message(content: &str) -> Vec<u8> {
    let header = format!("Content-Length: {}\r\n\r\n", content.len());
    let mut message = header.into_bytes();
    message.extend_from_slice(content.as_bytes());
    message
}

/// Helper to read LSP response
fn read_lsp_response(reader: &mut impl BufRead) -> Option<Value> {
    let mut headers = std::collections::HashMap::new();

    // Read headers
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }

        let line = line.trim_end();
        if line.is_empty() {
            break;
        }

        if let Some((key, value)) = line.split_once(": ") {
            headers.insert(key.to_string(), value.to_string());
        }
    }

    // Read content
    #[allow(clippy::collapsible_if)]
    if let Some(content_length) = headers.get("Content-Length") {
        if let Ok(length) = content_length.parse::<usize>() {
            let mut content = vec![0u8; length];
            reader.read_exact(&mut content).ok()?;
            return serde_json::from_slice(&content).ok();
        }
    }

    None
}

#[test]
fn test_lsp_initialize() {
    // Create channels for communication
    let (tx_in, _rx_in) = mpsc::channel::<Vec<u8>>();
    let (_tx_out, _rx_out) = mpsc::channel::<Vec<u8>>();

    // Mock stdin/stdout
    #[allow(dead_code)]
    struct MockIO {
        input: mpsc::Receiver<Vec<u8>>,
        output: mpsc::Sender<Vec<u8>>,
        buffer: Vec<u8>,
    }

    impl Read for MockIO {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.buffer.is_empty() {
                match self.input.recv_timeout(Duration::from_millis(100)) {
                    Ok(data) => self.buffer = data,
                    Err(_) => return Ok(0),
                }
            }

            let len = std::cmp::min(buf.len(), self.buffer.len());
            buf[..len].copy_from_slice(&self.buffer[..len]);
            self.buffer.drain(..len);
            Ok(len)
        }
    }

    impl Write for MockIO {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.output.send(buf.to_vec()).unwrap();
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // Test initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": "file:///test",
            "capabilities": {}
        }
    });

    // Send request
    tx_in.send(create_lsp_message(&init_request.to_string())).unwrap();

    // TODO: The current LspServer implementation expects real stdin/stdout
    // We need to refactor it to accept generic Read/Write traits for testing

    // For now, just verify the server can be created
    let _server = LspServer::new();
    // Server successfully created
}

#[test]
fn test_lsp_message_format() {
    // Test message formatting
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let message = create_lsp_message(content);
    let expected_header = format!("Content-Length: {}\r\n\r\n", content.len());
    let expected = [expected_header.as_bytes(), content.as_bytes()].concat();
    assert_eq!(&message[..], &expected[..]);
}

#[test]
fn test_lsp_response_parsing() {
    // Test response parsing - verify content length
    let json_content = r#"{"jsonrpc":"2.0","id":1,"result":{"test":true}}"#;
    let header = format!("Content-Length: {}\r\n\r\n", json_content.len());
    let response = [header.as_bytes(), json_content.as_bytes()].concat();
    let mut reader = BufReader::new(&response[..]);

    let parsed = read_lsp_response(&mut reader);
    assert!(parsed.is_some(), "Failed to parse LSP response");

    let value = parsed.unwrap();
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], 1);
    assert_eq!(value["result"]["test"], true);
}
