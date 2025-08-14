//! Sanity invariant tests for the LSP server
//!
//! These tests verify fundamental JSON-RPC and LSP protocol invariants
//! that should always hold regardless of server state.

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};
use serde_json::json;

/// Verify all responses have proper JSON-RPC structure
#[test]
fn test_response_structure_invariants() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Test successful response
    let response = send_request(
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

    // Must have jsonrpc field
    assert_eq!(
        response["jsonrpc"], "2.0",
        "Response must have jsonrpc: 2.0"
    );

    // Must have exactly one of result or error (XOR)
    let has_result = response.get("result").is_some();
    let has_error = response.get("error").is_some();
    assert!(
        has_result ^ has_error,
        "Response must have exactly one of result or error"
    );

    // Must have matching ID
    assert!(
        response.get("id").is_some(),
        "Response must have an id field"
    );
}

/// Verify error responses have proper structure
#[test]
fn test_error_response_structure() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send unknown method to trigger error
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/unknownMethod",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["error"].is_object());

    let error = &response["error"];
    assert!(error["code"].is_number(), "Error must have numeric code");
    assert!(
        error["message"].is_string(),
        "Error must have string message"
    );

    // Standard JSON-RPC error codes
    let code = error["code"].as_i64().unwrap();
    assert!(
        code == -32700  // Parse error
        || code == -32600  // Invalid request
        || code == -32601  // Method not found
        || code == -32602  // Invalid params
        || code == -32603  // Internal error
        || code < -32000, // Server-defined errors
        "Error code must be standard JSON-RPC or server-defined"
    );
}

/// Verify ID matching between request and response
#[test]
fn test_id_matching_invariants() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Test with number ID
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///test.pl"
                }
            }
        }),
    );
    assert_eq!(response["id"], 42, "Response ID must match request ID");

    // Test with string ID
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": "test-id",
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": "file:///test.pl"
                }
            }
        }),
    );
    assert_eq!(
        response["id"], "test-id",
        "Response ID must match string request ID"
    );
}

/// Verify diagnostics always include version
#[test]
fn test_diagnostics_version_invariant() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open document
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///diag.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = 1;\nmy $y = $undefined;"
                }
            }
        }),
    );

    // Wait for diagnostics
    if let Some(diag) = common::read_notification_method(
        &mut server,
        "textDocument/publishDiagnostics",
        common::short_timeout(),
    ) {
        assert!(
            diag["params"]["version"].is_number(),
            "Diagnostics must include version"
        );
        assert!(
            diag["params"]["diagnostics"].is_array(),
            "Diagnostics must be an array"
        );
    }

    // Fix the error
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///diag.pl",
                    "version": 2
                },
                "contentChanges": [
                    { "text": "my $x = 1;\nmy $y = 2;" }
                ]
            }
        }),
    );

    // Wait for clear diagnostics
    if let Some(diag) = common::read_notification_method(
        &mut server,
        "textDocument/publishDiagnostics",
        common::short_timeout(),
    ) {
        assert_eq!(
            diag["params"]["version"], 2,
            "Clear diagnostics must have updated version"
        );
        let diags = diag["params"]["diagnostics"].as_array().unwrap();
        assert!(
            diags.is_empty(),
            "Fixed code should have empty diagnostics array"
        );
    }
}

/// Verify server doesn't crash on malformed JSON
#[test]
fn test_no_crash_on_malformed_json() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send various malformed inputs
    common::send_raw_message(&mut server, "{invalid json");
    common::send_raw_message(&mut server, "null");
    common::send_raw_message(&mut server, "[]");
    common::send_raw_message(&mut server, "42");

    // Server should still be responsive
    let response = send_request(
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

/// Verify proper error for invalid method
#[test]
fn test_invalid_request_errors() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Unknown method should return -32601
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "unknownMethod",
            "params": {}
        }),
    );

    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601); // Method not found
}

/// Verify notifications don't produce responses
#[test]
fn test_notifications_no_response() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send notification (no ID field)
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///notif.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "print 'hello';"
                }
            }
        }),
    );

    // Should not get a response for notification
    let resp = common::read_response_timeout(&mut server, common::short_timeout());

    // We might get a diagnostics notification, but not a response
    if let Some(r) = resp {
        // If we got something, it should be a notification (no id field)
        assert!(
            r.get("id").is_none(),
            "Notifications should not produce responses with IDs"
        );
        assert_eq!(
            r.get("method").and_then(|v| v.as_str()),
            Some("textDocument/publishDiagnostics")
        );
    }
}

/// Verify server handles concurrent requests properly
#[test]
fn test_concurrent_request_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send multiple requests with different IDs
    for i in 200..205 {
        common::send_request_no_wait(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "textDocument/documentSymbol",
                "params": {
                    "textDocument": {
                        "uri": format!("file:///test{}.pl", i)
                    }
                }
            }),
        );
    }

    // Collect responses
    let mut received_ids = Vec::new();
    for _ in 0..5 {
        if let Some(resp) =
            common::read_response_timeout(&mut server, std::time::Duration::from_secs(5))
        {
            if let Some(id) = resp["id"].as_i64() {
                received_ids.push(id);
            }
        }
    }

    // Should receive all IDs (order may vary)
    received_ids.sort();
    assert_eq!(received_ids, vec![200, 201, 202, 203, 204]);
}
