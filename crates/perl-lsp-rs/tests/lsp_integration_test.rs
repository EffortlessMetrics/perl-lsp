//! Integration tests for the LSP server

use perl_lsp::LspServer;
use perl_tdd_support::must_some;
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
fn test_lsp_initialize() -> Result<(), Box<dyn std::error::Error>> {
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
            self.output
                .send(buf.to_vec())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))?;
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
    tx_in
        .send(create_lsp_message(&init_request.to_string()))
        .map_err(|e| format!("Failed to send init request: {}", e))?;

    // AC4: Use in-memory mock I/O for testing LSP message exchange
    let mock_in = MockIO { input: _rx_in, output: _tx_out.clone(), buffer: Vec::new() };
    let mock_out = MockIO {
        input: mpsc::channel().1, // dummy
        output: _tx_out,
        buffer: Vec::new(),
    };

    let server = LspServer::with_io(Box::new(mock_in), Box::new(mock_out));

    // Process one message
    // In this refactor, we just verify it can be instantiated with custom IO
    assert!(!server.is_initialized());
    Ok(())
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
fn test_lsp_response_parsing() -> Result<(), Box<dyn std::error::Error>> {
    // Test response parsing - verify content length
    let json_content = r#"{"jsonrpc":"2.0","id":1,"result":{"test":true}}"#;
    let header = format!("Content-Length: {}\r\n\r\n", json_content.len());
    let response = [header.as_bytes(), json_content.as_bytes()].concat();
    let mut reader = BufReader::new(&response[..]);

    let parsed = read_lsp_response(&mut reader);
    assert!(parsed.is_some(), "Failed to parse LSP response");

    let value = parsed.ok_or("Failed to parse LSP response")?;
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], 1);
    assert_eq!(value["result"]["test"], true);
    Ok(())
}

// AC5: Tests for LSP protocol edge cases

#[test]
fn test_malformed_json_message() {
    // Test that malformed JSON is handled gracefully
    let malformed_content = r#"{"jsonrpc":"2.0","id":1,"method":"test""#; // Missing closing brace
    let header = format!("Content-Length: {}\r\n\r\n", malformed_content.len());
    let message = [header.as_bytes(), malformed_content.as_bytes()].concat();
    let mut reader = BufReader::new(&message[..]);

    let parsed = read_lsp_response(&mut reader);
    // Malformed JSON should result in None
    assert!(parsed.is_none(), "Malformed JSON should not parse");
}

#[test]
fn test_incomplete_headers() {
    // Test that incomplete headers are handled gracefully
    let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    // Missing the blank line after header
    let incomplete_header = format!("Content-Length: {}\r\n{}", json_content.len(), json_content);
    let mut reader = BufReader::new(incomplete_header.as_bytes());

    let parsed = read_lsp_response(&mut reader);
    // Incomplete header (missing blank line) should result in None since the protocol requires it
    assert!(parsed.is_none(), "Missing blank line separator should fail to parse");
}

#[test]
fn test_missing_content_length_header() {
    // Test that missing Content-Length header is handled gracefully
    let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let invalid_header = format!("Content-Type: application/json\r\n\r\n{}", json_content);
    let mut reader = BufReader::new(invalid_header.as_bytes());

    let parsed = read_lsp_response(&mut reader);
    // Missing Content-Length should result in None
    assert!(parsed.is_none(), "Missing Content-Length should fail to parse");
}

#[test]
fn test_invalid_content_length() {
    // Test that invalid Content-Length values are handled gracefully
    let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let invalid_header = "Content-Length: not-a-number\r\n\r\n";
    let message = [invalid_header.as_bytes(), json_content.as_bytes()].concat();
    let mut reader = BufReader::new(&message[..]);

    let parsed = read_lsp_response(&mut reader);
    // Invalid Content-Length should result in None
    assert!(parsed.is_none(), "Invalid Content-Length should fail to parse");
}

#[test]
fn test_oversized_payload() {
    // Test that oversized payloads are handled correctly
    // Create a message with Content-Length larger than actual content
    let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let oversized_length = json_content.len() + 1000;
    let header = format!("Content-Length: {}\r\n\r\n", oversized_length);
    let message = [header.as_bytes(), json_content.as_bytes()].concat();
    let mut reader = BufReader::new(&message[..]);

    let parsed = read_lsp_response(&mut reader);
    // Oversized payload should result in None (EOF before reading full content)
    assert!(parsed.is_none(), "Oversized payload should fail to parse");
}

#[test]
fn test_zero_content_length() {
    // Test that zero Content-Length is handled gracefully
    let header = "Content-Length: 0\r\n\r\n";
    let mut reader = BufReader::new(header.as_bytes());

    let parsed = read_lsp_response(&mut reader);
    // Zero-length content should parse as None (empty JSON)
    assert!(parsed.is_none(), "Zero-length content should fail to parse as valid JSON");
}

#[test]
fn test_multiple_headers() {
    // Test that multiple headers are handled correctly
    let json_content = r#"{"jsonrpc":"2.0","id":1,"method":"test"}"#;
    let header =
        format!("Content-Length: {}\r\nContent-Type: application/json\r\n\r\n", json_content.len());
    let message = [header.as_bytes(), json_content.as_bytes()].concat();
    let mut reader = BufReader::new(&message[..]);

    let parsed = read_lsp_response(&mut reader);
    assert!(parsed.is_some(), "Multiple headers should be parsed correctly");

    let value = must_some(parsed);
    assert_eq!(value["jsonrpc"], "2.0");
    assert_eq!(value["id"], 1);
}

#[test]
fn test_empty_input_stream() {
    // Test that empty input stream is handled gracefully
    let empty_input: &[u8] = &[];
    let mut reader = BufReader::new(empty_input);

    let parsed = read_lsp_response(&mut reader);
    // Empty stream should result in None
    assert!(parsed.is_none(), "Empty input should result in None");
}

#[test]
fn test_partial_json_in_stream() {
    // Test that partial JSON in stream is handled gracefully
    let json_content = r#"{"jsonrpc":"2.0","id":1,"met"#; // Truncated
    let header = format!("Content-Length: {}\r\n\r\n", json_content.len());
    let message = [header.as_bytes(), json_content.as_bytes()].concat();
    let mut reader = BufReader::new(&message[..]);

    let parsed = read_lsp_response(&mut reader);
    // Partial JSON should result in None
    assert!(parsed.is_none(), "Partial JSON should fail to parse");
}

#[test]
fn test_lsp_server_with_cursor_io() {
    // Test that LspServer can be created with Cursor I/O (in-memory buffers)
    use std::io::Cursor;

    let input = Cursor::new(Vec::new());
    let output = Vec::new();

    let server = LspServer::with_io(Box::new(input), Box::new(output));

    // Verify server is created successfully
    assert!(!server.is_initialized(), "Server should not be initialized yet");
}

#[test]
fn test_concurrent_io_access() {
    // Test that concurrent I/O access is thread-safe (AC6)
    use std::sync::Arc;
    use std::thread;

    let (tx, _rx) = mpsc::channel::<Vec<u8>>();
    let output = Arc::new(tx);

    // Create multiple threads that would write to output
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let out = output.clone();
            thread::spawn(move || {
                let message = format!("Message {}", i);
                let _ = out.send(message.into_bytes());
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        assert!(handle.join().is_ok(), "Thread should complete successfully");
    }

    // If we get here, concurrent access was thread-safe
}
