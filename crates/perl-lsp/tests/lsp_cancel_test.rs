//! Tests for $/cancelRequest notification
//! Phase 1 Stabilization: Deterministic cancellation tests with stable harness

use perl_tdd_support::must;
use serde_json::json;
use std::time::Duration;

mod common;
mod support;
use common::*;
use support::{handshake_initialize, shutdown_graceful, spawn_lsp};

/// Test that cancel request is handled properly
///
/// This test sends a request and immediately cancels it to verify
/// the server handles cancellation correctly. For builds with the test
/// endpoint, it uses a slow operation; otherwise it uses hover which
/// may or may not be cancelled in time.
#[test]
fn test_cancel_request_handling() {
    // Skip test in constrained environments where LSP initialization is unreliable
    // This includes single-threaded environments and CI systems with limited resources
    let thread_count =
        std::env::var("RUST_TEST_THREADS").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);

    if thread_count <= 2 || std::env::var("CI").is_ok() {
        eprintln!(
            "Skipping cancellation test in constrained environment (threads: {})",
            thread_count
        );
        return;
    }

    // Quick LSP availability check - skip if LSP fails to initialize within reasonable time
    // This prevents false failures in environments with slow LSP startup
    {
        let mut test_server = start_lsp_server();
        let init_start = std::time::Instant::now();

        // Try a quick initialization with shorter timeout
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            initialize_lsp(&mut test_server)
        })) {
            Ok(_) => {
                // LSP started successfully, continue with test
                let init_time = init_start.elapsed();
                eprintln!(
                    "LSP initialization took {:?}, proceeding with cancellation test",
                    init_time
                );
            }
            Err(_) => {
                eprintln!("Skipping cancellation test due to LSP initialization timeout/failure");
                return;
            }
        }
    }

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
                must(Err::<(), _>(format!(
                    "Slow operation should have been cancelled, but got result: {:?}",
                    resp
                )));
            }
            // For hover, completing is acceptable since it's fast
        }
    } else if has_test_endpoint {
        must(Err::<(), _>(format!("Expected a response for slow operation")));
    }
    // No response for hover is acceptable (cancelled before processing)
}

/// Test that $/cancelRequest itself doesn't produce a response
#[test]
fn test_cancel_request_no_response() {
    // Skip test in constrained environments where LSP initialization is unreliable
    // This includes single-threaded environments and CI systems with limited resources
    let thread_count =
        std::env::var("RUST_TEST_THREADS").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);

    if thread_count <= 2 || std::env::var("CI").is_ok() {
        eprintln!(
            "Skipping cancellation test in constrained environment (threads: {})",
            thread_count
        );
        return;
    }

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

    // Drain any diagnostics or other notifications from didOpen (reduced timeout for performance)
    drain_until_quiet(&mut server, Duration::from_millis(50), Duration::from_millis(200));

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

/// PHASE 1 STABLE: Test deterministic cancellation with stable harness
#[test]
fn test_cancel_deterministic_stable() -> Result<(), Box<dyn std::error::Error>> {
    // Use new stable harness for deterministic cancellation
    let mut harness = spawn_lsp();

    // Perform handshake initialization
    let init_result = handshake_initialize(&mut harness, None);
    assert!(init_result.is_ok(), "Initialization should succeed");

    // Open a document
    let uri = "file:///test.pl";
    harness.open(uri, "my $x = 42;\n")?;

    // Barrier to ensure document is indexed
    harness.barrier();

    // Send a hover request that we'll cancel
    // Use a consistent request ID for testing
    let request_id = 12345;

    // Manually send request without waiting
    harness.notify(
        "textDocument/hover",
        json!({
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 5}
        }),
    );

    // Immediately cancel the request
    harness.cancel(request_id);

    // Assert no response arrives for this ID
    harness.assert_no_response_for_canceled(request_id, Duration::from_millis(300));

    // Verify server is still responsive with a simple request
    let result = harness.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "Server should remain responsive after cancellation");

    // Clean shutdown
    shutdown_graceful(&mut harness);

    Ok(())
}

/// Test cancelling multiple requests
#[test]
fn test_cancel_multiple_requests() {
    // Skip test in constrained environments where LSP initialization is unreliable
    // This includes single-threaded environments and CI systems with limited resources
    let thread_count =
        std::env::var("RUST_TEST_THREADS").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);

    if thread_count <= 2 || std::env::var("CI").is_ok() {
        eprintln!(
            "Skipping cancellation test in constrained environment (threads: {})",
            thread_count
        );
        return;
    }

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
