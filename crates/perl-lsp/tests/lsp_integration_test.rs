//! Integration tests for the LSP server

use perl_parser::LspServer;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use tempfile::tempfile;

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
    // Use real IO via temporary files to simulate stdin/stdout
    let mut input = tempfile().expect("create input tempfile");
    let mut output_reader = tempfile().expect("create output tempfile");
    let output_writer = output_reader
        .try_clone()
        .expect("clone tempfile for writing");
    let output = Arc::new(Mutex::new(Box::new(output_writer) as Box<dyn Write + Send>));
    let mut server = LspServer::with_output(output);

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": "file:///test",
            "capabilities": {},
        }
    });

    // Write request to input file and reset position
    input
        .write_all(&create_lsp_message(&init_request.to_string()))
        .unwrap();
    input.seek(SeekFrom::Start(0)).unwrap();

    // Handle the request and flush response to output file
    server.handle_message(&mut input).unwrap();

    // Read response from output file
    output_reader.seek(SeekFrom::Start(0)).unwrap();
    let mut reader = BufReader::new(output_reader);
    let response = read_lsp_response(&mut reader).expect("Failed to read LSP response");

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["capabilities"].is_object());
    assert_eq!(response["result"]["serverInfo"]["name"], "perl-lsp");
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

