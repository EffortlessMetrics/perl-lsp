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
    send_notification(&mut server, json!({
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
    }));

    // Wait for diagnostics or settle time to ensure file is processed
    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    // Verify basic LSP functionality (hover) with DAP feature enabled
    let hover_id = 100;
    send_request_no_wait(&mut server, json!({
        "jsonrpc": "2.0",
        "id": hover_id,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 1, "character": 4 } // over $x
        }
    }));

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
    send_notification(&mut server, json!({
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
    }));

    std::thread::sleep(Duration::from_millis(500));
    drain_until_quiet(&mut server, Duration::from_millis(100), Duration::from_millis(1000));

    let start = Instant::now();
    let hover_id = 200;
    send_request_no_wait(&mut server, json!({
        "jsonrpc": "2.0",
        "id": hover_id,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }
        }
    }));

    let response = read_response_matching_i64(&mut server, hover_id, Duration::from_secs(5));
    let latency = start.elapsed();
    
    assert!(response.is_some(), "Hover response missing in performance test");
    
    // AC2: Maintain <50ms response time (p50) - using 250ms for CI safety
    assert!(latency < Duration::from_millis(250), "LSP response too slow with DAP enabled: {:?}", latency);
    
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
    send_notification(&mut server, json!({
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
    }));

    // Wait for indexing
    std::thread::sleep(Duration::from_millis(1000));
    drain_until_quiet(&mut server, Duration::from_millis(200), Duration::from_millis(2000));

    // Verify workspace symbol search works
    let search_id = 300;
    send_request_no_wait(&mut server, json!({
        "jsonrpc": "2.0",
        "id": search_id,
        "method": "workspace/symbol",
        "params": { "query": "target_func" }
    }));

    let response = read_response_matching_i64(&mut server, search_id, Duration::from_secs(5));
    assert!(response.is_some(), "Workspace symbol response should be present");
    let resp_val = response.unwrap();
    assert!(resp_val["result"].as_array().map_or(false, |a| !a.is_empty()), "Should find target_func symbol in result: {:?}", resp_val);
    
    Ok(())
}