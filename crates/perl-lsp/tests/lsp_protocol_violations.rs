use serde_json::json;
use std::io::Write;
use std::time::Duration;

mod common;
#[allow(unused_imports)]
use common::{
    initialize_lsp, read_response, read_response_timeout, send_notification, send_request,
    short_timeout, start_lsp_server,
};

/// Comprehensive protocol violation tests
/// Tests all possible ways the LSP protocol can be violated
// Run with: cargo test -p perl-lsp-rs --features strict-jsonrpc
#[cfg(feature = "strict-jsonrpc")]
#[test]
fn test_missing_jsonrpc_version() {
    let server = start_lsp_server();

    // Send request without jsonrpc field
    send_request(
        &server,
        json!({
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600); // Invalid Request
}

#[test]
fn test_wrong_jsonrpc_version() {
    let server = start_lsp_server();

    // Send request with wrong version
    send_request(
        &server,
        json!({
            "jsonrpc": "1.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
}

#[test]
fn test_notification_with_id() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Notifications should not have an id field
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,  // Invalid for notification
            "method": "$/cancelRequest",
            "params": {"id": 999}
        }),
    );

    // Server should handle gracefully
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_request_without_id() {
    let server = start_lsp_server();

    // Requests must have an id field
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(100));
    // Server should treat as notification
}

#[cfg(feature = "strict-jsonrpc")]
#[test]
fn test_duplicate_request_ids() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Send two requests with same ID
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 100,  // Duplicate ID
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    // Should handle both, but may cause confusion
    let response1 = read_response(&server);
    let response2 = read_response(&server);
    assert_eq!(response1["id"], 100);
    assert_eq!(response2["id"], 100);
}

#[test]
fn test_invalid_content_length_header() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    // Send malformed content-length
    server
        .stdin_writer()
        .write_all(b"Content-Length: not-a-number\r\n\r\n{\"jsonrpc\":\"2.0\"}")?;
    server.stdin_writer().flush()?;

    std::thread::sleep(Duration::from_millis(100));
    // Server should recover
    Ok(())
}

#[test]
fn test_mismatched_content_length() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    // Content-Length doesn't match actual content
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let wrong_length = content.len() + 100;

    server
        .stdin_writer()
        .write_all(format!("Content-Length: {}\r\n\r\n{}", wrong_length, content).as_bytes())?;
    server.stdin_writer().flush()?;

    std::thread::sleep(Duration::from_millis(100));
    // Server should handle gracefully
    Ok(())
}

#[test]
fn test_missing_content_length_header() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    // Send without Content-Length
    server
        .stdin_writer()
        .write_all(b"\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}")?;
    server.stdin_writer().flush()?;

    std::thread::sleep(Duration::from_millis(100));
    // Server should reject
    Ok(())
}

#[test]
fn test_additional_headers() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    // Send with additional headers
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;

    server.stdin_writer().write_all(
        format!(
            "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\nX-Custom: test\r\n\r\n{}",
            content.len(),
            content
        ).as_bytes()
    )?;
    server.stdin_writer().flush()?;

    let response = read_response(&server);
    assert!(response["id"].is_number());
    Ok(())
}

#[test]
fn test_invalid_utf8_in_message() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Try to send invalid UTF-8
    let mut invalid_content = Vec::from(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"file:///test.pl\",\"languageId\":\"perl\",\"version\":1,\"text\":\"");
    invalid_content.push(0xFF); // Invalid UTF-8 byte
    invalid_content.push(0xFE); // Invalid UTF-8 byte
    invalid_content.extend_from_slice(b"\"}}}");

    server
        .stdin_writer()
        .write_all(format!("Content-Length: {}\r\n\r\n", invalid_content.len()).as_bytes())?;
    server.stdin_writer().write_all(&invalid_content)?;
    server.stdin_writer().flush()?;

    std::thread::sleep(Duration::from_millis(100));
    // Server should handle invalid UTF-8
    Ok(())
}

#[cfg(feature = "strict-jsonrpc")]
#[test]
fn test_request_before_initialization() {
    let server = start_lsp_server();

    // Try to use server before initialization
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 0}
            }
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32002); // Server not initialized
}

#[test]
fn test_double_initialization() {
    let server = start_lsp_server();

    // Initialize once
    initialize_lsp(&server);

    // Try to initialize again
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": null,
                "capabilities": {}
            }
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
}

#[test]
fn test_invalid_method_name_format() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Test various invalid method names
    let invalid_methods = [
        "",
        " ",
        "123",
        "method with spaces",
        "method/with/too/many/slashes",
        "/startingWithSlash",
        "endingWithSlash/",
        "special!chars",
        "unicode/методъ",
    ];

    for (i, method) in invalid_methods.iter().enumerate() {
        // Fix: capture the response returned by send_request
        let response = send_request(
            &server,
            json!({
                "jsonrpc": "2.0",
                "id": i + 100,
                "method": method,
                "params": {}
            }),
        );

        // Verify the server returns METHOD_NOT_FOUND error
        assert!(
            response["error"].is_object(),
            "Expected error for method '{}'",
            method
        );
        assert_eq!(
            response["error"]["code"], -32601,
            "Expected METHOD_NOT_FOUND (-32601) for method '{}', got {:?}",
            method, response["error"]["code"]
        );
    }
}

#[test]
fn test_params_type_violations() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Params should be object or array, not scalar
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": "string params"  // Invalid
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());

    // Number params
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": 123  // Invalid
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
}

#[test]
fn test_circular_json_reference() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Create a JSON string that would cause circular reference if parsed incorrectly
    let circular_json = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.pl",
                "languageId": "perl",
                "version": 1,
                "text": "my $self = \\$self;"
            }
        }
    }"#;

    send_request(&server, serde_json::from_str(circular_json)?);

    // Should handle without stack overflow
    std::thread::sleep(Duration::from_millis(100));
    Ok(())
}

#[test]
fn test_extremely_nested_json() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Create deeply nested structure
    let mut nested = json!(null);
    for _ in 0..1000 {
        nested = json!({"nested": nested});
    }

    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "workspace/executeCommand",
            "params": {
                "command": "test",
                "arguments": [nested]
            }
        }),
    );

    // Should handle without stack overflow
    let response = read_response(&server);
    assert!(response["error"].is_object() || response["result"].is_object());
}

#[test]
fn test_null_values_in_required_fields() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Send nulls where objects are expected
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": null,  // Should be object
                "position": null       // Should be object
            }
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
}

#[test]
fn test_wrong_type_for_position() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Position with wrong types
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {
                    "line": "zero",      // Should be number
                    "character": "five"  // Should be number
                }
            }
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
}

#[test]
fn test_negative_positions() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///test.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'hello';"
                }
            }
        }),
    );

    // Negative line and character
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": -1, "character": -1}
            }
        }),
    );

    let response = read_response(&server);
    // Should handle gracefully
    assert!(response.is_object());
}

#[test]
fn test_float_positions() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Positions with floating point numbers
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 1.5, "character": 2.7}
            }
        }),
    );

    let response = read_response(&server);
    // Should truncate or error
    assert!(response.is_object());
}

#[test]
fn test_invalid_uri_schemes() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let invalid_uris = vec![
        "not-a-uri",
        "://missing-scheme.pl",
        "file//missing-colon.pl",
        "file:missing-slashes.pl",
        "javascript:alert('xss')",
        "data:text/plain,hello",
        "../../../etc/passwd",
        "\\\\unc\\path\\file.pl",
    ];

    for uri in invalid_uris {
        send_notification(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "perl",
                        "version": 1,
                        "text": "print 'test';"
                    }
                }
            }),
        );

        // Should handle invalid URIs gracefully
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn test_response_without_request() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Send a response without a corresponding request
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 999,
            "result": {"some": "data"}
        }),
    );

    // Server should ignore or handle gracefully
    std::thread::sleep(Duration::from_millis(100));
}

#[cfg(feature = "strict-jsonrpc")]
#[test]
fn test_batch_request_violations() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    // Empty batch
    server
        .stdin_writer()
        .write_all(b"Content-Length: 2\r\n\r\n[]")?;
    server.stdin_writer().flush()?;

    std::thread::sleep(Duration::from_millis(100));

    // Batch with mixed valid/invalid
    let batch = json!([
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
        {"invalid": "request"},
        {"jsonrpc": "2.0", "id": 2, "method": "shutdown"}
    ]);

    let content = batch.to_string();
    server
        .stdin_writer()
        .write_all(format!("Content-Length: {}\r\n\r\n{}", content.len(), content).as_bytes())?;
    server.stdin_writer().flush()?;

    // Should process valid ones and error on invalid
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

#[test]
fn test_incomplete_message() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    // Send partial message
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}"#; // Missing closing brace

    server.stdin_writer().write_all(
        format!("Content-Length: {}\r\n\r\n{}", content.len() + 1, content).as_bytes(),
    )?;
    server.stdin_writer().flush()?;

    // Server should timeout or error
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

#[test]
fn test_mixed_protocol_versions() {
    let server = start_lsp_server();

    // Initialize with 2.0
    initialize_lsp(&server);

    // Then send 1.0 style request
    send_request(
        &server,
        json!({
            "jsonrpc": "1.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": []
        }),
    );

    let response = read_response(&server);
    assert!(response["error"].is_object());
}

#[test]
fn test_method_result_and_error() {
    let server = start_lsp_server();

    // Response with both result and error (invalid)
    send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"data": "test"},
            "error": {"code": -32000, "message": "Error"}
        }),
    );

    // Server should handle this protocol violation
    std::thread::sleep(Duration::from_millis(100));
}
