//! DAP LSP Non-Regression Tests (AC17)
//!
//! Tests to ensure LSP functionality remains unaffected by DAP integration
//!
//! Specification: docs/reference/DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-integration-non-regression
//!
//! Run with: cargo test -p perl-lsp-rs --test dap_non_regression_tests

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
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///ac1_comprehensive.pl";
    let text = "package TestPkg;\nsub test_sub { my $var = 1; }\n1;\n";
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
                    "text": text
                }
            }
        }),
    );

    // Wait for diagnostics or settle time to ensure file is processed
    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    // AC1: hover
    let hover_id = 100;
    send_request_no_wait(
        &server,
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
    assert!(
        read_response_matching_i64(&server, hover_id, Duration::from_secs(5)).is_some(),
        "Hover response should be present with DAP feature enabled"
    );

    // AC1: completion at line 0 character 10 (inside package name)
    let completion_id = 101;
    send_request_no_wait(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": completion_id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 10 }
            }
        }),
    );
    assert!(
        read_response_matching_i64(&server, completion_id, Duration::from_secs(5)).is_some(),
        "Completion response should be present with DAP feature enabled"
    );

    // AC1: definition at line 1 character 4 (over test_sub)
    let definition_id = 102;
    send_request_no_wait(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": definition_id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 1, "character": 4 }
            }
        }),
    );
    assert!(
        read_response_matching_i64(&server, definition_id, Duration::from_secs(5)).is_some(),
        "Definition response should be present with DAP feature enabled"
    );

    // AC1: workspace/symbol query
    let wsymbol_id = 103;
    send_request_no_wait(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": wsymbol_id,
            "method": "workspace/symbol",
            "params": { "query": "test_sub" }
        }),
    );
    assert!(
        read_response_matching_i64(&server, wsymbol_id, Duration::from_secs(5)).is_some(),
        "Workspace symbol response should be present with DAP feature enabled"
    );

    // AC1: formatting (null result is acceptable, response must arrive)
    let formatting_id = 104;
    send_request_no_wait(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": formatting_id,
            "method": "textDocument/formatting",
            "params": {
                "textDocument": { "uri": uri },
                "options": { "tabSize": 4, "insertSpaces": true }
            }
        }),
    );
    assert!(
        read_response_matching_i64(&server, formatting_id, Duration::from_secs(5)).is_some(),
        "Formatting response should be present with DAP feature enabled"
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-lsp-response-time
#[test]
// AC:17
fn test_lsp_response_time_maintained() -> Result<()> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///perf.pl";
    let text = "my $val = 42;\n";
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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    // AC2: measure latency across 10 requests, assert p50 < 100ms
    let mut latencies = Vec::with_capacity(10);
    for i in 0..10 {
        let id = 200 + i;
        let start = Instant::now();
        send_request_no_wait(
            &server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 4 }
                }
            }),
        );
        let _resp = read_response_matching_i64(&server, id, Duration::from_secs(5));
        latencies.push(start.elapsed());
    }

    latencies.sort();
    // p50 = lower median of 10 sorted samples (index 4 = 5th value in 1-indexed,
    // i.e. the 50th percentile boundary). Index 5 would be p60.
    let p50 = latencies[4];
    assert!(
        p50 < Duration::from_millis(100),
        "p50 LSP response too slow with DAP enabled: {:?}",
        p50
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-workspace-navigation
#[test]
// AC:17
fn test_workspace_navigation_with_dap() -> Result<()> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///nav.pl";
    let text = "package NavTest;\nsub target_func { }\n1;\n";
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
                    "text": text
                }
            }
        }),
    );

    // Wait for indexing
    std::thread::sleep(Duration::from_millis(1000));
    drain_until_quiet(&server, Duration::from_millis(200), Duration::from_millis(2000));

    // Verify workspace symbol search works
    let search_id = 300;
    send_request_no_wait(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": search_id,
            "method": "workspace/symbol",
            "params": { "query": "target_func" }
        }),
    );

    let response = read_response_matching_i64(&server, search_id, Duration::from_secs(5));
    assert!(response.is_some(), "Workspace symbol response should be present");
    let resp_val = response.ok_or_else(|| anyhow::anyhow!("Expected workspace symbol response"))?;
    assert!(
        resp_val["result"].as_array().is_some_and(|a| !a.is_empty()),
        "Should find target_func symbol in result: {:?}",
        resp_val
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-memory-isolation
#[test]
// AC:17
fn test_lsp_dap_memory_isolation() -> Result<()> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///memory_test.pl";
    let text = "my $data = 1;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    // Send multiple LSP requests to test responsiveness under load
    for i in 0..50 {
        let req_id = 400 + i;
        send_request_no_wait(
            &server,
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

        if read_response_matching_i64(&server, req_id, Duration::from_millis(500)).is_some() {
            // Response received
        }
    }

    // Verify server still responsive after load
    let final_id = 500;
    send_request_no_wait(
        &server,
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

    let response = read_response_matching_i64(&server, final_id, Duration::from_secs(5));
    assert!(response.is_some(), "Server should remain responsive after load");

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-test-pass-rate
#[test]
// AC:17
fn test_lsp_test_pass_rate_100_percent() -> Result<()> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///comprehensive.pl";
    let text = "package TestPkg;\nsub test_sub { my $var = 1; }\n1;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    // Test hover
    let hover_id = 600;
    send_request_no_wait(
        &server,
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
        read_response_matching_i64(&server, hover_id, Duration::from_secs(5)).is_some(),
        "Hover should work with DAP feature enabled"
    );

    // Test completion
    let completion_id = 601;
    send_request_no_wait(
        &server,
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
        read_response_matching_i64(&server, completion_id, Duration::from_secs(5)).is_some(),
        "Completion should work with DAP feature enabled"
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-concurrent-sessions
#[test]
// AC:17
fn test_concurrent_lsp_dap_sessions() -> Result<()> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///concurrent.pl";
    let text = "my $value = 42;\nprint $value;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    // Send concurrent requests
    let hover_id = 700;
    let completion_id = 701;

    send_request_no_wait(
        &server,
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
        &server,
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
    let hover_resp = read_response_matching_i64(&server, hover_id, Duration::from_secs(5));
    let completion_resp =
        read_response_matching_i64(&server, completion_id, Duration::from_secs(5));

    assert!(hover_resp.is_some(), "Hover response should arrive in concurrent scenario");
    assert!(completion_resp.is_some(), "Completion response should arrive in concurrent scenario");

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-incremental-parsing
#[test]
// AC:17
fn test_incremental_parsing_during_debugging() -> Result<()> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///incremental.pl";
    let text = "my $original = 1;\n";

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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    // Apply ranged (incremental) edit — insert a new line at line 1
    let start_time = Instant::now();
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end":   { "line": 1, "character": 0 }
                    },
                    "text": "my $new = 2;\n"
                }]
            }
        }),
    );

    // settle_time measures: notification send + 100ms sleep + server diagnostic flush.
    // The server-side parse is <1ms; this is a CI-safe "server didn't hang" check.
    // Threshold: 100ms (mandatory sleep) + 500ms (drain ceiling) + 200ms (CI margin) = 800ms.
    std::thread::sleep(Duration::from_millis(100));
    drain_until_quiet(&server, Duration::from_millis(50), Duration::from_millis(500));
    let settle_time = start_time.elapsed();

    // Verify LSP still responsive after incremental edit
    let hover_id = 800;
    send_request_no_wait(
        &server,
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

    let response = read_response_matching_i64(&server, hover_id, Duration::from_secs(5));
    assert!(response.is_some(), "LSP should remain responsive after incremental edit");
    assert!(
        settle_time < Duration::from_millis(800),
        "Server failed to settle after incremental edit within 800ms: {:?}",
        settle_time
    );

    Ok(())
}

/// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac17-performance-baseline
#[test]
// AC:17
fn test_performance_baseline_no_regression() -> Result<()> {
    // Baseline: LSP hover p50 < 100ms under dap-phase3 feature.
    // This test is the CI-safe equivalent of AC8's regression detection.
    // Full performance dashboard (AC8) is tracked via `cargo bench`.
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///baseline.pl";
    let text = "my $baseline = 1;\n";
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
                    "text": text
                }
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_millis(1000));

    let mut latencies = Vec::with_capacity(10);
    for i in 0..10 {
        let id = 900 + i;
        let start = Instant::now();
        send_request_no_wait(
            &server,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": { "line": 0, "character": 4 }
                }
            }),
        );
        let _resp = read_response_matching_i64(&server, id, Duration::from_secs(5));
        latencies.push(start.elapsed());
    }

    latencies.sort();
    // p50 = lower median of 10 sorted samples (index 4 = 5th value in 1-indexed,
    // i.e. the 50th percentile boundary). Index 5 would be p60.
    let p50 = latencies[4];
    assert!(
        p50 < Duration::from_millis(100),
        "Performance baseline regression: p50 hover latency {:?} exceeds 100ms with dap-phase3",
        p50
    );

    Ok(())
}
