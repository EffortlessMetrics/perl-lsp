use serde_json::json;
use std::io::Write;
use std::time::Duration;

mod common;
use common::{initialize_lsp, read_response, send_notification, send_request, start_lsp_server};

/// Comprehensive protocol violation tests
/// Tests all possible ways the LSP protocol can be violated

#[test]
fn test_missing_jsonrpc_version() {
    let mut server = start_lsp_server();

    // Send request without jsonrpc field
    send_request(
        &mut server,
        json!({
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32600); // Invalid Request
}

#[test]
fn test_wrong_jsonrpc_version() {
    let mut server = start_lsp_server();

    // Send request with wrong version
    send_request(
        &mut server,
        json!({
            "jsonrpc": "1.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_notification_with_id() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Notifications should not have an id field
    send_request(
        &mut server,
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
    let mut server = start_lsp_server();

    // Requests must have an id field
    send_request(
        &mut server,
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

#[test]
fn test_duplicate_request_ids() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send two requests with same ID
    send_request(
        &mut server,
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
        &mut server,
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
    let response1 = read_response(&mut server);
    let response2 = read_response(&mut server);
    assert_eq!(response1["id"], 100);
    assert_eq!(response2["id"], 100);
}

#[test]
fn test_invalid_content_length_header() {
    let mut server = start_lsp_server();

    // Send malformed content-length
    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"Content-Length: not-a-number\r\n\r\n{\"jsonrpc\":\"2.0\"}")
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    // Server should recover
}

#[test]
fn test_mismatched_content_length() {
    let mut server = start_lsp_server();

    // Content-Length doesn't match actual content
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let wrong_length = content.len() + 100;

    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(format!("Content-Length: {}\r\n\r\n{}", wrong_length, content).as_bytes())
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    // Server should handle gracefully
}

#[test]
fn test_missing_content_length_header() {
    let mut server = start_lsp_server();

    // Send without Content-Length
    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}")
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    // Server should reject
}

#[test]
fn test_additional_headers() {
    let mut server = start_lsp_server();

    // Send with additional headers
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;

    server.stdin.as_mut().unwrap().write_all(
        format!(
            "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\nX-Custom: test\r\n\r\n{}",
            content.len(),
            content
        ).as_bytes()
    ).unwrap();

    let response = read_response(&mut server);
    assert!(response["id"].is_number());
}

#[test]
fn test_invalid_utf8_in_message() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Try to send invalid UTF-8
    let mut invalid_content = Vec::from(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"textDocument/didOpen\",\"params\":{\"textDocument\":{\"uri\":\"file:///test.pl\",\"languageId\":\"perl\",\"version\":1,\"text\":\"");
    invalid_content.push(0xFF); // Invalid UTF-8 byte
    invalid_content.push(0xFE); // Invalid UTF-8 byte
    invalid_content.extend_from_slice(b"\"}}}");

    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(format!("Content-Length: {}\r\n\r\n", invalid_content.len()).as_bytes())
        .unwrap();
    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&invalid_content)
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));
    // Server should handle invalid UTF-8
}

#[test]
fn test_request_before_initialization() {
    let mut server = start_lsp_server();

    // Try to use server before initialization
    send_request(
        &mut server,
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

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32002); // Server not initialized
}

#[test]
fn test_double_initialization() {
    let mut server = start_lsp_server();

    // Initialize once
    initialize_lsp(&mut server);

    // Try to initialize again
    send_request(
        &mut server,
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

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_invalid_method_name_format() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Test various invalid method names
    let invalid_methods = vec![
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
        send_request(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i + 100,
                "method": method,
                "params": {}
            }),
        );

        let response = read_response(&mut server);
        assert!(response["error"].is_object());
    }
}

#[test]
fn test_params_type_violations() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Params should be object or array, not scalar
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": "string params"  // Invalid
        }),
    );

    let response = read_response(&mut server);
    assert!(response["error"].is_object());

    // Number params
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": 123  // Invalid
        }),
    );

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_circular_json_reference() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

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

    send_request(&mut server, serde_json::from_str(circular_json).unwrap());

    // Should handle without stack overflow
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_extremely_nested_json() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Create deeply nested structure
    let mut nested = json!(null);
    for _ in 0..1000 {
        nested = json!({"nested": nested});
    }

    send_request(
        &mut server,
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
    let response = read_response(&mut server);
    assert!(response["error"].is_object() || response["result"].is_object());
}

#[test]
fn test_null_values_in_required_fields() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send nulls where objects are expected
    send_request(
        &mut server,
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

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_wrong_type_for_position() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Position with wrong types
    send_request(
        &mut server,
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

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_negative_positions() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    send_notification(
        &mut server,
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
        &mut server,
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

    let response = read_response(&mut server);
    // Should handle gracefully
    assert!(response.is_object());
}

#[test]
fn test_float_positions() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Positions with floating point numbers
    send_request(
        &mut server,
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

    let response = read_response(&mut server);
    // Should truncate or error
    assert!(response.is_object());
}

#[test]
fn test_invalid_uri_schemes() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

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
            &mut server,
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
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send a response without a corresponding request
    send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 999,
            "result": {"some": "data"}
        }),
    );

    // Server should ignore or handle gracefully
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_batch_request_violations() {
    let mut server = start_lsp_server();

    // Empty batch
    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"Content-Length: 2\r\n\r\n[]")
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));

    // Batch with mixed valid/invalid
    let batch = json!([
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
        {"invalid": "request"},
        {"jsonrpc": "2.0", "id": 2, "method": "shutdown"}
    ]);

    let content = batch.to_string();
    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(format!("Content-Length: {}\r\n\r\n{}", content.len(), content).as_bytes())
        .unwrap();

    // Should process valid ones and error on invalid
    std::thread::sleep(Duration::from_millis(200));
}

#[test]
fn test_incomplete_message() {
    let mut server = start_lsp_server();

    // Send partial message
    let content = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}"#; // Missing closing brace

    server
        .stdin
        .as_mut()
        .unwrap()
        .write_all(format!("Content-Length: {}\r\n\r\n{}", content.len() + 1, content).as_bytes())
        .unwrap();

    // Server should timeout or error
    std::thread::sleep(Duration::from_millis(500));
}

#[test]
fn test_mixed_protocol_versions() {
    let mut server = start_lsp_server();

    // Initialize with 2.0
    initialize_lsp(&mut server);

    // Then send 1.0 style request
    send_request(
        &mut server,
        json!({
            "jsonrpc": "1.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": []
        }),
    );

    let response = read_response(&mut server);
    assert!(response["error"].is_object());
}

#[test]
fn test_method_result_and_error() {
    let mut server = start_lsp_server();

    // Response with both result and error (invalid)
    send_request(
        &mut server,
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
