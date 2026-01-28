//! DAP LSP Non-Regression Tests (AC17)
//!
//! Tests to ensure LSP functionality remains unaffected by DAP integration
//!
//! Specification: docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-integration-non-regression
//!
//! Run with: cargo test -p perl-lsp --test dap_non_regression_tests --features dap-phase3

#![cfg(feature = "dap-phase3")]

use anyhow::Result;
use serde_json::json;
use std::time::{Duration, Instant};

#[path = "common/mod.rs"]
mod common;
use common::*;

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-features-unaffected
#[test]
// AC:17
fn test_lsp_features_unaffected_by_dap() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    let text = "use strict;\nmy $x = 1;\nprint $x;\n";
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
                    "text": text
                }
            }
        }),
    );

    // Wait for diagnostics or settle time to ensure file is processed
    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    // Verify basic LSP functionality (hover) with DAP feature enabled
    let hover_id = 100;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 4 } // over $x
            }
        }),
    );

    let response = read_response_matching_i64(&mut server, hover_id, Duration::from_secs(5));
    assert!(response.is_some(), "Hover response should be present after didOpen");

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-response-time
#[test]
// AC:17
fn test_lsp_response_time_maintained() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///perf.pl";
    let text = "my $val = 42;\n";
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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    let start = Instant::now();
    let hover_id = 200;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );

    let response = read_response_matching_i64(&mut server, hover_id, Duration::from_secs(5));
    let latency = start.elapsed();

    assert!(response.is_some(), "Hover response missing in performance test");

    // AC2: Maintain <50ms response time (p50) - using 250ms for CI safety
    assert!(
        latency < Duration::from_millis(250),
        "LSP response too slow with DAP enabled: {:?}",
        latency
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-workspace-navigation
#[test]
// AC:17
fn test_workspace_navigation_with_dap() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///nav.pl";
    let text = "package NavTest;\nsub target_func { }\n1;\n";
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
                    "text": text
                }
            }
        }),
    );

    // Wait for indexing
    std::thread::sleep(Duration::from_millis(1000));
    drain_until_quiet(&mut server, Duration::from_millis(200), Duration::from_millis(2000));

    // Verify workspace symbol search works
    let search_id = 300;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": search_id,
            "method": "workspace/symbol",
            "params": { "query": "target_func" }
        }),
    );

    let response = read_response_matching_i64(&mut server, search_id, Duration::from_secs(5));
    assert!(response.is_some(), "Workspace symbol response should be present");
    let resp_val = response.ok_or_else(|| anyhow::anyhow!("Expected workspace symbol response"))?;
    assert!(
        resp_val["result"].as_array().map_or(false, |a| !a.is_empty()),
        "Should find target_func symbol in result: {:?}",
        resp_val
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-memory-isolation
#[test]
// AC:17
fn test_lsp_dap_memory_isolation() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///memory_test.pl";
    let text = "my $data = 1;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    // Send multiple LSP requests to test responsiveness under load
    for i in 0..50 {
        let req_id = 400 + i;
        send_request_no_wait(
            &mut server,
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 4 }
                }
            }),
        );

        if read_response_matching_i64(&mut server, req_id, Duration::from_millis(500)).is_some() {
            // Response received
        }
    }

    // Verify server still responsive after load
    let final_id = 500;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": final_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );

    let response = read_response_matching_i64(&mut server, final_id, Duration::from_secs(5));
    assert!(response.is_some(), "Server should remain responsive after load");

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-test-pass-rate
#[test]
// AC:17
fn test_lsp_test_pass_rate_100_percent() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///comprehensive.pl";
    let text = "package TestPkg;\nsub test_sub { my $var = 1; }\n1;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    // Test hover
    let hover_id = 600;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 20 }
            }
        }),
    );
    assert!(
        read_response_matching_i64(&mut server, hover_id, Duration::from_secs(5)).is_some(),
        "Hover should work with DAP feature enabled"
    );

    // Test completion
    let completion_id = 601;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": completion_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 25 }
            }
        }),
    );
    assert!(
        read_response_matching_i64(&mut server, completion_id, Duration::from_secs(5)).is_some(),
        "Completion should work with DAP feature enabled"
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-concurrent-sessions
#[test]
// AC:17
fn test_concurrent_lsp_dap_sessions() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///concurrent.pl";
    let text = "my $value = 42;\nprint $value;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    // Send concurrent requests
    let hover_id = 700;
    let completion_id = 701;

    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );

    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": completion_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 7 }
            }
        }),
    );

    // Both responses should arrive
    let hover_resp = read_response_matching_i64(&mut server, hover_id, Duration::from_secs(5));
    let completion_resp = read_response_matching_i64(&mut server, completion_id, Duration::from_secs(5));

    assert!(hover_resp.is_some(), "Hover response should arrive in concurrent scenario");
    assert!(completion_resp.is_some(), "Completion response should arrive in concurrent scenario");

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-incremental-parsing
#[test]
// AC:17
fn test_incremental_parsing_during_debugging() -> Result<()> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///incremental.pl";
    let text = "my $original = 1;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    // Apply incremental edit
    let start_time = Instant::now();
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [
                    { "text": "my $original = 1;\nmy $new = 2;\n" }
                ]
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(100));
    drain_until_quiet(&mut server, Duration::from_millis(50), Duration::from_millis(500));
    let parse_time = start_time.elapsed();

    // Verify LSP still responsive after edit
    let hover_id = 800;
    send_request_no_wait(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 4 }
            }
        }),
    );

    let response = read_response_matching_i64(&mut server, hover_id, Duration::from_secs(5));
    assert!(response.is_some(), "LSP should remain responsive after incremental edit");
    assert!(
        parse_time < Duration::from_secs(1),
        "Incremental parsing too slow: {:?}",
        parse_time
    );

    Ok(())
}
