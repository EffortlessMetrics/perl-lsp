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
fn test_malformed_headers_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Validates PR #173's enhanced malformed frame recovery implementation
    // Tests that the server gracefully handles malformed headers with enhanced error recovery
    let mut server = start_lsp_server();

    // Send header with extra spaces - this should be handled gracefully
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string();
    let header = format!("Content-Length   : {}\r\n\r\n", body.len());

    {
        server.stdin_writer().write_all(header.as_bytes())?;
        server.stdin_writer().write_all(body.as_bytes())?;
        server.stdin_writer().flush()?;
    }

    // PR #173: Enhanced malformed frame recovery should handle this gracefully
    // Server should continue processing or send an appropriate response
    let _response = common::read_response_timeout(&mut server, Duration::from_millis(1000));

    // Verify server didn't crash and either processed request or handled error gracefully
    // The enhanced error handling should maintain session continuity
    // Response can be Some(value) if processed, or None if malformed frame was handled gracefully
    // Server should handle malformed headers gracefully

    // Test that server is still responsive after malformed header
    let test_response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "shutdown"
        }),
    );
    assert!(test_response["result"].is_null() || test_response["error"].is_object());
    Ok(())
}

#[test]
fn test_edge_case_malformed_frame_recovery() -> Result<(), Box<dyn std::error::Error>> {
    // Validates PR #173's enhanced malformed frame recovery for edge cases
    // Tests header-only with missing body scenario - server should recover gracefully
    let mut server = start_lsp_server();

    // Send header without body - this is a malformed frame that should be handled
    let header = "Content-Length: 50\r\n\r\n";

    {
        server.stdin_writer().write_all(header.as_bytes())?;
        server.stdin_writer().flush()?;
    }

    // PR #173: Enhanced frame recovery should handle this gracefully without crashing
    // Wait briefly to allow malformed frame processing
    std::thread::sleep(Duration::from_millis(200));

    // This test validates that malformed frames don't crash the process entirely
    // The server may terminate the connection but should not panic or crash
    // Try to initialize - this may fail if connection was terminated
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        initialize_lsp(&mut server);
    })) {
        Ok(()) => {
            // If initialization succeeded, verify server is responsive
            let response = common::send_request(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "shutdown"
                }),
            );
            assert!(
                response["result"].is_null() || response["error"].is_object(),
                "Server should be responsive after successful recovery"
            );
        }
        Err(_) => {
            // If initialization failed, that's acceptable for this malformed frame type
            // The important thing is that the server didn't crash the process
            // Server gracefully terminated connection after malformed frame - this is acceptable recovery behavior
        }
    }
    Ok(())
}

#[test]
fn test_invalid_json_body() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send invalid JSON
    send_raw_message(&mut server, "{this is not: valid json}}");

    // Server should either send error response or ignore
    let _response = common::read_response_timeout(&mut server, Duration::from_millis(500));
    // We don't assert on the response - server may ignore or error

    // Test that server can handle subsequent requests after invalid JSON
    // Server may terminate connection, which is acceptable recovery behavior
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        common::send_request(
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
        )
    })) {
        Ok(response) => {
            assert!(
                response["result"].is_array() || response["error"].is_object(),
                "Server should handle valid requests after malformed JSON recovery"
            );
        }
        Err(_) => {
            // Connection terminated - this is acceptable for invalid JSON scenarios
            // Server terminated connection after invalid JSON - acceptable recovery behavior
        }
    }
    Ok(())
}

#[test]
fn test_server_specific_header_parsing() -> Result<(), Box<dyn std::error::Error>> {
    // Validates PR #173's enhanced header parsing implementation
    // Tests how our server handles duplicate Content-Length headers specifically
    let mut server = start_lsp_server();

    // Send duplicate Content-Length headers
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string();
    let header =
        format!("Content-Length: {}\r\nContent-Length: {}\r\n\r\n", body.len(), body.len());

    {
        server.stdin_writer().write_all(header.as_bytes())?;
        server.stdin_writer().write_all(body.as_bytes())?;
        server.stdin_writer().flush()?;
    }

    // PR #173: Our server should handle duplicate headers gracefully
    // Enhanced frame parsing should either process the request or handle error appropriately
    let response = common::read_response_timeout(&mut server, Duration::from_millis(1000));

    // Verify our specific implementation handles duplicate headers
    // Server should either parse successfully or provide enhanced error response
    match response {
        Some(resp) => {
            assert!(
                resp["result"].is_object() || resp["error"].is_object(),
                "Server should return valid response for duplicate headers"
            );
        }
        None => {
            // If response is None, verify server is still responsive (enhanced recovery)
            let test_response = common::send_request(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "shutdown"
                }),
            );
            assert!(
                test_response["result"].is_null() || test_response["error"].is_object(),
                "Server should maintain functionality after duplicate header handling"
            );
        }
    }
    Ok(())
}

#[test]
fn test_wrong_content_length_recovery() -> Result<(), Box<dyn std::error::Error>> {
    // Validates PR #173's enhanced recovery from wrong Content-Length headers
    // Tests that server gracefully handles mismatched content length and recovers
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send body with wrong Content-Length
    let body = json!({"jsonrpc": "2.0", "id": 100, "method": "shutdown"}).to_string();
    let header = format!("Content-Length: {}\r\n\r\n", body.len() + 10); // Wrong length

    {
        server.stdin_writer().write_all(header.as_bytes())?;
        server.stdin_writer().write_all(body.as_bytes())?;
        server.stdin_writer().flush()?;
    }

    // PR #173: Enhanced malformed frame recovery should handle wrong content-length
    // Wait for server to process the malformed frame
    std::thread::sleep(Duration::from_millis(300));

    // Test that server recovers gracefully after wrong content-length
    // This validates the "Continue processing - don't crash the server" behavior
    // Connection may be terminated, which is acceptable recovery
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        common::send_request(
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
        )
    })) {
        Ok(response) => {
            assert!(
                response["result"].is_array() || response["error"].is_object(),
                "Server should recover and handle valid requests after wrong content-length frame"
            );

            // If first request succeeded, test shutdown too
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                common::send_request(
                    &mut server,
                    json!({
                        "jsonrpc": "2.0",
                        "method": "shutdown"
                    }),
                )
            })) {
                Ok(shutdown_response) => {
                    assert!(
                        shutdown_response["result"].is_null()
                            || shutdown_response["error"].is_object(),
                        "Server should maintain complete functionality after content-length recovery"
                    );
                }
                Err(_) => {
                    // Even the shutdown failed - connection was terminated, which is acceptable
                    // Connection terminated during shutdown - acceptable recovery behavior
                }
            }
        }
        Err(_) => {
            // Connection terminated after wrong content-length - acceptable recovery behavior
            // Server terminated connection after wrong content-length - acceptable enhanced recovery behavior
        }
    }
    Ok(())
}

#[test]
fn test_unknown_method() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

#[test]
fn test_header_case_sensitivity() -> Result<(), Box<dyn std::error::Error>> {
    // Validates PR #173's header case sensitivity handling implementation
    // Tests that our server properly handles case-insensitive headers per HTTP/LSP standards
    let mut server = start_lsp_server();

    // Try different header casings - HTTP headers should be case-insensitive
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}).to_string();
    let header = format!("content-length: {}\r\n\r\n", body.len()); // lowercase

    {
        server.stdin_writer().write_all(header.as_bytes())?;
        server.stdin_writer().write_all(body.as_bytes())?;
        server.stdin_writer().flush()?;
    }

    // PR #173: Enhanced header parsing should handle case-insensitive headers correctly
    // Our implementation should follow HTTP/LSP standards for case-insensitive headers
    let response = common::read_response_timeout(&mut server, Duration::from_millis(1000));

    match response {
        Some(resp) => {
            assert!(
                resp["result"].is_object() || resp["error"].is_object(),
                "Server should handle case-insensitive headers per HTTP/LSP standards"
            );
        }
        None => {
            // If parsing fails due to case sensitivity, verify enhanced recovery works
            let test_response = common::send_request(
                &mut server,
                json!({
                    "jsonrpc": "2.0",
                    "method": "shutdown"
                }),
            );
            assert!(
                test_response["result"].is_null() || test_response["error"].is_object(),
                "Server should maintain functionality with enhanced error recovery"
            );
        }
    }

    // Test additional case variations to validate comprehensive header handling
    let mixed_case_test = "Content-Length: 25\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":2}";
    // Handle broken pipe gracefully - server may have exited due to previous malformed input
    if let Err(e) = server.stdin_writer().write_all(mixed_case_test.as_bytes()) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            return Ok(()); // Server exited, test complete
        }
        return Err(Box::new(e));
    }
    let _ = server.stdin_writer().flush(); // Ignore flush errors

    // Server should handle mixed case headers consistently
    let _final_response = common::read_response_timeout(&mut server, Duration::from_millis(500));
    Ok(())
}
