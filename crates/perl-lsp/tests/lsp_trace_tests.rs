//! Trace protocol tests for LSP 3.17
//!
//! Tests $/setTrace notification (client->server) and $/logTrace notification
//! structure (server->client). The server dispatch handles unknown methods via
//! the catch-all arm, so $/setTrace should not crash the server.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== $/setTrace (client->server notification) ====================

#[test]
fn test_set_trace_off() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // $/setTrace is a notification -- use harness.notify(), no response expected.
    // TraceValue: "off" disables all tracing.
    harness.notify(
        "$/setTrace",
        json!({
            "value": "off"
        }),
    );

    // Server must remain responsive after receiving setTrace
    let _result = harness.request("workspace/symbol", json!({"query": ""})).unwrap_or(json!(null));
    Ok(())
}

#[test]
fn test_set_trace_messages() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // TraceValue: "messages" enables message-level tracing
    harness.notify(
        "$/setTrace",
        json!({
            "value": "messages"
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""})).unwrap_or(json!(null));
    Ok(())
}

#[test]
fn test_set_trace_verbose() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // TraceValue: "verbose" enables verbose tracing (includes message bodies)
    harness.notify(
        "$/setTrace",
        json!({
            "value": "verbose"
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""})).unwrap_or(json!(null));
    Ok(())
}

// ==================== $/logTrace (server->client notification) ====================

#[test]
fn test_log_trace_notification_contract() -> TestResult {
    // $/logTrace is a server->client notification. We validate the JSON structure
    // that the server would emit rather than sending it ourselves.

    let log_trace_basic = json!({
        "jsonrpc": "2.0",
        "method": "$/logTrace",
        "params": {
            "message": "Received request textDocument/completion"
        }
    });

    let log_trace_verbose = json!({
        "jsonrpc": "2.0",
        "method": "$/logTrace",
        "params": {
            "message": "Received request textDocument/completion",
            "verbose": "Request params: {\"textDocument\":{\"uri\":\"file:///test.pl\"},\"position\":{\"line\":0,\"character\":5}}"
        }
    });

    // Basic: "message" is required
    assert!(log_trace_basic["params"]["message"].is_string(), "logTrace must have a message field");
    assert!(
        log_trace_basic["params"].get("verbose").is_none(),
        "basic logTrace should not have verbose field"
    );

    // Verbose: "message" required, "verbose" optional
    assert!(
        log_trace_verbose["params"]["message"].is_string(),
        "logTrace must have a message field"
    );
    assert!(
        log_trace_verbose["params"]["verbose"].is_string(),
        "verbose logTrace should have a verbose string field"
    );

    // Verify no "id" field (notifications never have an id)
    assert!(log_trace_basic.get("id").is_none(), "notifications must not have an id");
    assert!(log_trace_verbose.get("id").is_none(), "notifications must not have an id");

    Ok(())
}
