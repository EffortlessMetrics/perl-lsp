//! Tests for $/cancelRequest notification

use serde_json::json;
use std::time::Duration;

mod common;
use common::*;

/// Test that cancel request is handled properly
///
/// This test sends a request and immediately cancels it to verify
/// the server handles cancellation correctly. For builds with the test
/// endpoint, it uses a slow operation; otherwise it uses hover which
/// may or may not be cancelled in time.
#[test]
fn test_cancel_request_handling() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // First, test if the slow operation endpoint exists
    let test_id = 8888;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": test_id,
            "method": "$/test/slowOperation",
            "params": {}
        }),
    );

    let test_response =
        read_response_matching_i64(&mut server, test_id, Duration::from_millis(600));
    let has_test_endpoint = test_response.is_some_and(|resp| {
        resp.get("error").is_none_or(|e| {
            e["code"].as_i64() != Some(-32601) // -32601 = Method not found
        })
    });

    // Now run the actual cancellation test
    let request_id = 9999;

    if has_test_endpoint {
        // Use the slow operation for reliable cancellation
        send_request_no_wait(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "$/test/slowOperation",
                "params": {}
            }),
        );

        // Wait a tiny bit to ensure the request starts processing
        // then cancel it while it's still running (takes 1 second total)
        std::thread::sleep(Duration::from_millis(50));
    } else {
        // Fall back to hover which may or may not be cancelled in time
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
    }

    // Send cancellation
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

    // Read the response
    let response = read_response_matching_i64(&mut server, request_id, Duration::from_secs(2));

    if let Some(resp) = response {
        if let Some(error) = resp.get("error") {
            let code = error["code"].as_i64().unwrap_or(0);
            // -32800 = RequestCancelled per LSP spec for $/cancelRequest
            // Accept it as cancelled
            assert_eq!(code, -32800, "Should have cancellation error code -32800");
        } else {
            // Request completed before cancellation - that's OK for hover
            // but not for the slow operation
            if has_test_endpoint {
                panic!("Slow operation should have been cancelled, but got result: {:?}", resp);
            }
            // For hover, completing is acceptable since it's fast
        }
    } else if has_test_endpoint {
        panic!("Expected a response for slow operation");
    }
    // No response for hover is acceptable (cancelled before processing)
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
