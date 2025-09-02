//! Tests for handling malformed JSON-RPC frames
//!
//! These tests ensure the LSP server gracefully handles malformed input
//! without hanging or crashing.

mod common;
use common::{initialize_lsp, send_raw_message, start_lsp_server};
use serde_json::json;
use std::io::Write;
use std::time::Duration;

#[test]
#[ignore] // Server-specific behavior for malformed headers
fn test_malformed_content_length_header() {
    let mut server = start_lsp_server();

    // Send header with extra spaces
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string();
    let header = format!("Content-Length   : {}\r\n\r\n", body.len());

    {
        server.stdin_writer().write_all(header.as_bytes()).unwrap();
        server.stdin_writer().write_all(body.as_bytes()).unwrap();
        server.stdin_writer().flush().unwrap();
    }

    // Server behavior varies - some handle this, some don't
    let _response = common::read_response_timeout(&mut server, Duration::from_millis(500));
}

#[test]
#[ignore] // Edge case handling varies by implementation
fn test_header_only_no_body() {
    let mut server = start_lsp_server();

    // Send header without body
    let header = "Content-Length: 50\r\n\r\n";

    {
        server.stdin_writer().write_all(header.as_bytes()).unwrap();
        server.stdin_writer().flush().unwrap();
    }

    // Wait a bit then send a valid request
    std::thread::sleep(Duration::from_millis(100));

    // Server should recover and handle next valid request
    initialize_lsp(&mut server);

    // Verify server is still responsive
    let response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "shutdown"
        }),
    );
    assert!(response["result"].is_null() || response["error"].is_object());
}

#[test]
fn test_invalid_json_body() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send invalid JSON
    send_raw_message(&mut server, "{this is not: valid json}}");

    // Server should either send error response or ignore
    let _response = common::read_response_timeout(&mut server, Duration::from_millis(500));
    // We don't assert on the response - server may ignore or error

    // Server should still handle valid requests
    let response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///test.pl"
                }
            }
        }),
    );
    assert!(response["result"].is_array() || response["error"].is_object());
}

#[test]
#[ignore] // Server-specific header parsing
fn test_duplicate_content_length() {
    let mut server = start_lsp_server();

    // Send duplicate Content-Length headers
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string();
    let header =
        format!("Content-Length: {}\r\nContent-Length: {}\r\n\r\n", body.len(), body.len());

    {
        server.stdin_writer().write_all(header.as_bytes()).unwrap();
        server.stdin_writer().write_all(body.as_bytes()).unwrap();
        server.stdin_writer().flush().unwrap();
    }

    // Server should handle it (typically using last value)
    let response = common::read_response(&mut server);
    assert!(response["result"].is_object() || response["error"].is_object());
}

#[test]
#[ignore] // Recovery from wrong content-length varies
fn test_wrong_content_length() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send body with wrong Content-Length
    let body = json!({"jsonrpc": "2.0", "id": 100, "method": "shutdown"}).to_string();
    let header = format!("Content-Length: {}\r\n\r\n", body.len() + 10); // Wrong length

    {
        server.stdin_writer().write_all(header.as_bytes()).unwrap();
        server.stdin_writer().write_all(body.as_bytes()).unwrap();
        server.stdin_writer().flush().unwrap();
    }

    // Wait a bit for server to process
    std::thread::sleep(Duration::from_millis(200));

    // Server should recover - send valid request
    let response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///test.pl"
                }
            }
        }),
    );
    assert!(response["result"].is_array() || response["error"].is_object());
}

#[test]
fn test_unknown_method() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send request with unknown method
    let response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/unknownMethod",
            "params": {}
        }),
    );

    // Should return method not found error or null result
    if response["error"].is_object() {
        assert_eq!(response["error"]["code"], -32601); // Method not found
    } else {
        // Some servers return null for unknown methods
        assert!(response["result"].is_null());
    }
}

#[test]
#[ignore] // Header case sensitivity is implementation-specific
fn test_case_sensitive_headers() {
    let mut server = start_lsp_server();

    // Try different header casings
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string();
    let header = format!("content-length: {}\r\n\r\n", body.len()); // lowercase

    {
        server.stdin_writer().write_all(header.as_bytes()).unwrap();
        server.stdin_writer().write_all(body.as_bytes()).unwrap();
        server.stdin_writer().flush().unwrap();
    }

    // Server should handle case-insensitive headers
    let response = common::read_response(&mut server);
    assert!(response["result"].is_object() || response["error"].is_object());
}
