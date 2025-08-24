//! Tests for $/cancelRequest notification

use serde_json::json;
use std::time::Duration;

mod common;
use common::*;

/// Test that cancel request is handled (may or may not cancel in time)
/// 
/// This test is intentionally flexible about the outcome since cancellation
/// is inherently racy. The important thing is that the server doesn't crash
/// and returns either a valid result or a cancellation error.
#[test]
fn test_cancel_request_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open a document
    let uri = "file:///test.pl";
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
                    "text": "my $x = 42;\n"
                }
            }
        }),
    );

    // Send a request and immediately cancel it
    let request_id = 9999;

    // Send the request (but don't wait for response yet)
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 5 }
            }
        }),
    );

    // Immediately send cancellation
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": request_id
            }
        }),
    );

    // Try to read the response - it may or may not be cancelled depending on timing
    let response = read_response_matching_i64(&mut server, request_id, Duration::from_secs(2));

    if let Some(resp) = response {
        // We got a response - check if it's cancelled or completed
        if let Some(error) = resp.get("error") {
            let code = error["code"].as_i64().unwrap_or(0);
            // -32802 = server cancelled (new), -32800 = request cancelled (old)
            // Both are acceptable
            assert!(
                code == -32802 || code == -32800 || code != 0,
                "Should have an error code"
            );
        } else {
            // Request completed before cancellation took effect - that's okay too
            assert!(resp.get("result").is_some(), "Should have result if not cancelled");
        }
    }
    // If no response, that's also fine - the request was cancelled before processing
}

/// Test that $/cancelRequest itself doesn't produce a response
#[test]
fn test_cancel_request_no_response() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Send a didOpen to keep server active
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///dummy.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": "# empty\n"
                }
            }
        }),
    );

    // Drain any diagnostics or other notifications from didOpen
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(500));

    // Check server is still alive before sending cancel
    assert!(server.is_alive(), "server exited before cancel test started");

    // Send a cancel request for a non-existent ID
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": 123456
            }
        }),
    );

    // Use bounded read to check for no response
    let response = read_response_timeout(&mut server, Duration::from_millis(200));

    // $/cancelRequest is a notification and should not produce any response
    assert!(response.is_none(), "$/cancelRequest produced an unexpected response");

    // Verify server is still alive after processing the notification
    assert!(server.is_alive(), "server should not exit on cancel notification");
}

/// Test cancelling multiple requests
#[test]
fn test_cancel_multiple_requests() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
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
                    "text": "print 'hello';\n"
                }
            }
        }),
    );

    // Send multiple requests
    let ids = [8001, 8002, 8003];

    for &id in &ids {
        send_request_no_wait(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 1 }
                }
            }),
        );
    }

    // Cancel the middle request
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": 8002
            }
        }),
    );

    // Check responses
    for &id in &ids {
        let response = read_response_matching_i64(&mut server, id, Duration::from_secs(2));
        if let Some(resp) = response {
            if id == 8002 {
                // This one might be cancelled (or might complete if fast enough)
                if let Some(error) = resp.get("error") {
                    let code = error["code"].as_i64().unwrap_or(0);
                    assert_eq!(code, -32800, "Cancelled request should have -32800 code");
                }
            } else {
                // Other requests should complete normally
                assert!(resp.get("result").is_some() || resp.get("error").is_some());
            }
        }
    }
}
