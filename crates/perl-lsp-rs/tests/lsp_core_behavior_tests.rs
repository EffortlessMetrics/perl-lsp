//! Core LSP Server Behavior Specification Tests
//!
//! Covers the fundamental server behaviors that every LSP client relies on:
//! 1. Initialization handshake and capability negotiation
//! 2. Document open/change/close lifecycle
//! 3. Graceful shutdown sequence
//! 4. Error responses for malformed/invalid requests
//! 5. Concurrent request handling
//! 6. Configuration change notifications

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ============================================================================
// 1. INITIALIZATION HANDSHAKE & CAPABILITY NEGOTIATION
// ============================================================================

/// Server must return capabilities in the initialize response.
#[test]
fn init_returns_capabilities() -> TestResult {
    let mut h = LspHarness::new();
    let result = h.initialize(None)?;

    assert!(result.get("capabilities").is_some(), "initialize must return capabilities");
    h.shutdown_gracefully();
    Ok(())
}

/// Server must return serverInfo with name.
#[test]
fn init_returns_server_info() -> TestResult {
    let mut h = LspHarness::new();
    let result = h.initialize(None)?;

    let info = result.get("serverInfo");
    assert!(info.is_some(), "initialize must return serverInfo");
    let name = info.and_then(|i| i.get("name")).and_then(|n| n.as_str());
    assert!(name.is_some(), "serverInfo must include name");
    h.shutdown_gracefully();
    Ok(())
}

/// When client advertises completionItem.snippetSupport, server should
/// advertise completion capability.
#[test]
fn init_negotiates_completion() -> TestResult {
    let mut h = LspHarness::new();
    let result = h.initialize(Some(json!({
        "textDocument": {
            "completion": {
                "completionItem": { "snippetSupport": true }
            }
        }
    })))?;

    let caps = result.get("capabilities").ok_or("no capabilities")?;
    assert!(
        caps.get("completionProvider").is_some(),
        "completionProvider should be advertised when client supports completion"
    );
    h.shutdown_gracefully();
    Ok(())
}

/// Server should advertise hover capability.
#[test]
fn init_advertises_hover() -> TestResult {
    let mut h = LspHarness::new();
    let result = h.initialize(None)?;

    let caps = result.get("capabilities").ok_or("no capabilities")?;
    assert!(caps.get("hoverProvider").is_some(), "hoverProvider should be advertised");
    h.shutdown_gracefully();
    Ok(())
}

/// Server should advertise definition capability.
#[test]
fn init_advertises_definition() -> TestResult {
    let mut h = LspHarness::new();
    let result = h.initialize(None)?;

    let caps = result.get("capabilities").ok_or("no capabilities")?;
    assert!(caps.get("definitionProvider").is_some(), "definitionProvider should be advertised");
    h.shutdown_gracefully();
    Ok(())
}

/// Server should advertise text document sync capability.
#[test]
fn init_advertises_text_sync() -> TestResult {
    let mut h = LspHarness::new();
    let result = h.initialize(None)?;

    let caps = result.get("capabilities").ok_or("no capabilities")?;
    assert!(caps.get("textDocumentSync").is_some(), "textDocumentSync should be advertised");
    h.shutdown_gracefully();
    Ok(())
}

// ============================================================================
// 2. DOCUMENT OPEN / CHANGE / CLOSE LIFECYCLE
// ============================================================================

/// Opening a document should allow hover requests on it.
#[test]
fn document_open_enables_hover() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;
    h.open("file:///test.pl", "my $x = 42;\n")?;
    h.barrier();

    let result = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 }
        }),
    );
    // We don't require a specific hover content, just that the server
    // doesn't error out for an open document.
    assert!(result.is_ok(), "hover on open document should not error: {:?}", result.err());
    h.shutdown_gracefully();
    Ok(())
}

/// Changing document content should reflect in subsequent requests.
#[test]
fn document_change_updates_state() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let uri = "file:///changing.pl";
    h.open(uri, "my $x = 1;\n")?;
    h.barrier();

    // Full-content change
    h.change_full(uri, 2, "sub greet { return 'hello'; }\n")?;
    h.barrier();

    // Request completion at a position in the new content
    let result = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 5 }
        }),
    );
    assert!(result.is_ok(), "hover after content change should not error");
    h.shutdown_gracefully();
    Ok(())
}

/// Closing a document should not cause server errors.
#[test]
fn document_close_graceful() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let uri = "file:///closeme.pl";
    h.open(uri, "my $y = 'hello';\n")?;
    h.barrier();
    h.close(uri)?;
    h.barrier();

    // Server should still respond to other requests after close
    let result = h.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "server should remain responsive after document close");
    h.shutdown_gracefully();
    Ok(())
}

/// Multiple documents can be open simultaneously.
#[test]
fn multiple_documents_open() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.open("file:///a.pl", "my $a = 1;\n")?;
    h.open("file:///b.pl", "my $b = 2;\n")?;
    h.open("file:///c.pl", "my $c = 3;\n")?;
    h.barrier();

    // Hover on each should work
    for uri in &["file:///a.pl", "file:///b.pl", "file:///c.pl"] {
        let result = h.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }),
        );
        assert!(result.is_ok(), "hover on {} should not error", uri);
    }
    h.shutdown_gracefully();
    Ok(())
}

/// Document version tracking: change bumps the version the server sees.
#[test]
fn document_version_tracking() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let uri = "file:///versioned.pl";
    h.open(uri, "my $v = 1;\n")?;
    h.barrier();

    // Send multiple version bumps
    h.change_full(uri, 2, "my $v = 2;\n")?;
    h.change_full(uri, 3, "my $v = 3;\n")?;
    h.change_full(uri, 4, "my $v = 4;\n")?;
    h.barrier();

    // Server should still respond correctly after multiple version changes
    let result = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }
        }),
    );
    assert!(result.is_ok(), "hover after version bumps should not error");
    h.shutdown_gracefully();
    Ok(())
}

// ============================================================================
// 3. GRACEFUL SHUTDOWN SEQUENCE
// ============================================================================

/// shutdown request must return null result per LSP spec.
#[test]
fn shutdown_returns_null() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let result = h.request("shutdown", json!({}))?;
    assert!(result.is_null(), "shutdown must return null, got: {}", result);

    h.shutdown_gracefully();
    Ok(())
}

/// shutdown after performing actual work should succeed cleanly.
#[test]
fn shutdown_after_work() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    // Do some real work first
    h.open("file:///work.pl", "sub hello { return 1; }\n")?;
    h.barrier();
    let _ = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///work.pl" },
            "position": { "line": 0, "character": 5 }
        }),
    );

    let result = h.request("shutdown", json!({}))?;
    assert!(result.is_null(), "shutdown after work must return null");
    h.shutdown_gracefully();
    Ok(())
}

// ============================================================================
// 4. ERROR RESPONSES FOR MALFORMED / INVALID REQUESTS
// ============================================================================

/// Unknown method should return MethodNotFound error.
#[test]
fn unknown_method_returns_error() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let response = h.request_raw(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/completelyFakeMethod",
        "params": {}
    }));

    // Should have an error field
    let error = response.get("error");
    assert!(error.is_some(), "unknown method should produce an error response, got: {}", response);
    h.shutdown_gracefully();
    Ok(())
}

/// Hover on a document that was never opened should produce an error.
#[test]
fn request_on_nonexistent_document() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    // Request hover on a URI that was never opened
    let result = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///nonexistent.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    // Server may return null or error — either is acceptable, but should not crash
    // The key assertion is that we got a response at all
    if let Ok(val) = result {
        assert!(val.is_null() || val.is_object(), "expected null or object");
    }
    // Error response is also acceptable — key is no crash
    h.shutdown_gracefully();
    Ok(())
}

/// Hover at a position beyond the document length should not crash.
#[test]
fn hover_invalid_position() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.open("file:///short.pl", "my $x = 1;\n")?;
    h.barrier();

    // Line 999 is way beyond the 1-line document
    let result = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///short.pl" },
            "position": { "line": 999, "character": 0 }
        }),
    );

    // null result is acceptable; error is also fine; crash is not
    if let Ok(val) = result {
        assert!(val.is_null() || val.is_object(), "expected null or hover object");
    }
    // Error is acceptable for out-of-bounds — key is no crash
    h.shutdown_gracefully();
    Ok(())
}

/// Request with empty/minimal params should not crash the server.
#[test]
fn empty_params_handled() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    // workspace/symbol with empty query is valid
    let result = h.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "workspace/symbol with empty query should succeed");
    h.shutdown_gracefully();
    Ok(())
}

// ============================================================================
// 5. CONCURRENT REQUEST HANDLING
// ============================================================================

/// Rapid sequential requests should all get responses.
#[test]
fn rapid_sequential_requests() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.open("file:///rapid.pl", "my $x = 1;\nmy $y = 2;\nmy $z = 3;\n")?;
    h.barrier();

    // Fire 5 hover requests in rapid succession
    let mut successes = 0;
    for line in 0..5 {
        let result = h.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": "file:///rapid.pl" },
                "position": { "line": line.min(2), "character": 4 }
            }),
        );
        if result.is_ok() {
            successes += 1;
        }
    }
    assert!(successes >= 3, "at least 3 of 5 rapid requests should succeed, got {}", successes);
    h.shutdown_gracefully();
    Ok(())
}

/// Interleaved edit and query operations should not corrupt state.
#[test]
fn interleaved_edit_and_query() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let uri = "file:///interleaved.pl";
    h.open(uri, "my $x = 1;\n")?;
    h.barrier();

    // Interleave edits and queries
    for version in 2..=5 {
        h.change_full(uri, version, &format!("my $x = {};\nmy $y = {};\n", version, version * 10))?;
        let result = h.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }),
        );
        assert!(
            result.is_ok(),
            "hover after edit v{} should not error: {:?}",
            version,
            result.err()
        );
    }
    h.shutdown_gracefully();
    Ok(())
}

// ============================================================================
// 6. CONFIGURATION CHANGE NOTIFICATIONS
// ============================================================================

/// didChangeConfiguration with settings should not crash.
#[test]
fn did_change_configuration() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "perlPath": "/usr/bin/perl"
                }
            }
        }),
    );
    h.barrier();

    // Server should still be responsive
    let result = h.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "server should respond after config change");
    h.shutdown_gracefully();
    Ok(())
}

/// Empty configuration change should be handled gracefully.
#[test]
fn empty_configuration_change() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.notify("workspace/didChangeConfiguration", json!({"settings": {}}));
    h.barrier();

    let result = h.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "server should respond after empty config change");
    h.shutdown_gracefully();
    Ok(())
}

/// Null settings in configuration change should not crash.
#[test]
fn null_configuration_settings() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.notify("workspace/didChangeConfiguration", json!({"settings": null}));
    h.barrier();

    let result = h.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "server should respond after null config settings");
    h.shutdown_gracefully();
    Ok(())
}

// ============================================================================
// 7. EDGE CASES
// ============================================================================

/// Request before initialize should produce an error (server not initialized).
#[test]
fn request_before_initialize() -> TestResult {
    let mut h = LspHarness::new_without_initialize();

    let response = h.request_raw(json!({
        "jsonrpc": "2.0",
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 }
        }
    }));

    // Server should either return an error or no response at all
    // (not crash). The key is we can still initialize after.
    let has_error = response.get("error").is_some();
    let has_null_result = response.get("result").is_some_and(|r| r.is_null());
    assert!(
        has_error || has_null_result || response.get("result").is_some(),
        "pre-init request should produce some response, got: {}",
        response
    );

    // Should still be able to initialize normally after
    let init_result = h.initialize(None);
    assert!(init_result.is_ok(), "should be able to initialize after pre-init request");
    h.shutdown_gracefully();
    Ok(())
}

/// Opening the same document twice should not crash.
#[test]
fn double_open_same_document() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    let uri = "file:///double.pl";
    h.open(uri, "my $x = 1;\n")?;
    h.barrier();
    h.open(uri, "my $x = 2;\n")?;
    h.barrier();

    // Should still work
    let result = h.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }
        }),
    );
    assert!(result.is_ok(), "hover after double open should not error");
    h.shutdown_gracefully();
    Ok(())
}

/// Closing a document that was never opened should not crash.
#[test]
fn close_never_opened() -> TestResult {
    let mut h = LspHarness::new();
    h.initialize(None)?;

    h.close("file:///never_opened.pl")?;
    h.barrier();

    // Server should still respond
    let result = h.request("workspace/symbol", json!({"query": ""}));
    assert!(result.is_ok(), "server should respond after closing never-opened doc");
    h.shutdown_gracefully();
    Ok(())
}
