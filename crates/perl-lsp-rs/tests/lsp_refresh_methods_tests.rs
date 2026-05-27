//! Tests for LSP server→client refresh requests
//!
//! These tests verify that the server correctly sends refresh requests to the
//! client for various features (code lenses, semantic tokens, diagnostics, etc.)

mod support;

use perl_lsp::LspServer;
use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Test that refresh requests succeed when client doesn't support them (no-op behavior)
#[test]
fn lsp_refresh_code_lens_not_sent_without_support() {
    let server = LspServer::new();
    // Default client capabilities don't support refresh - should be no-op
    assert!(server.request_code_lens_refresh().is_ok());
}

#[test]
fn lsp_refresh_semantic_tokens_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_semantic_tokens_refresh().is_ok());
}

#[test]
fn lsp_refresh_inlay_hint_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_inlay_hint_refresh().is_ok());
}

#[test]
fn lsp_refresh_inline_value_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_inline_value_refresh().is_ok());
}

#[test]
fn lsp_refresh_diagnostic_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_diagnostic_refresh().is_ok());
}

#[test]
fn lsp_refresh_folding_range_not_sent_without_support() {
    let server = LspServer::new();
    assert!(server.request_folding_range_refresh().is_ok());
}

#[test]
fn lsp_refresh_folding_range_sent_with_client_support() -> TestResult {
    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(
        "file:///workspace",
        Some(json!({
            "workspace": {
                "foldingRange": {
                    "refreshSupport": true
                }
            }
        })),
    )?;

    harness.notify("workspace/didChangeConfiguration", json!({ "settings": { "perl": {} } }));

    let requests = harness.drain_server_requests(500);
    let request = requests
        .iter()
        .find(|request| {
            request.get("method").and_then(Value::as_str) == Some("workspace/foldingRange/refresh")
        })
        .ok_or_else(|| {
            format!("expected workspace/foldingRange/refresh request, got {requests:?}")
        })?;

    assert_eq!(request.get("jsonrpc").and_then(Value::as_str), Some("2.0"));
    let id = request
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("folding range refresh missing integer id: {request}"))?;
    assert!((1..=i64::from(i32::MAX)).contains(&id), "request id out of bounds: {id}");
    assert_eq!(
        request.get("params"),
        Some(&Value::Null),
        "folding range refresh should send null params: {request}"
    );

    Ok(())
}
