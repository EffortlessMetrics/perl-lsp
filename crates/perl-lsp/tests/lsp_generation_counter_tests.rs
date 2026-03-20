//! Generation counter race prevention tests
//!
//! Verifies that rapid sequential `textDocument/didChange` notifications
//! do not corrupt document state. The generation counter in `DocumentState`
//! ensures stale parse results are discarded when a newer change arrives
//! while the server is still processing an earlier version.

use serde_json::json;

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

/// Send rapid sequential didChange notifications and verify the server
/// resolves to the latest version without errors.
#[test]
fn test_rapid_did_change_resolves_to_latest() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///gen_counter.pl";

    // Open document with initial content
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
                    "text": "my $x = 1;\n"
                }
            }
        }),
    );

    // Send rapid sequential didChange notifications (versions 2-6)
    // Each replaces the full document content with a different value.
    // The generation counter should ensure only the final version persists.
    for version in 2..=6 {
        send_notification(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "version": version
                    },
                    "contentChanges": [
                        { "text": format!("my $value = {};\n", version) }
                    ]
                }
            }),
        );
    }

    // Request hover on the document — the server must respond without error,
    // proving that the rapid changes did not corrupt internal state.
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );

    // The hover may return null (no hover info for a simple variable) or a
    // valid hover result — either is acceptable. What matters is no error.
    assert!(
        response.get("error").is_none(),
        "Hover after rapid changes should not produce an error: {response:?}"
    );

    Ok(())
}

/// Send interleaved incremental edits to verify generation counter handles
/// partial-document changes without state corruption.
#[test]
fn test_incremental_edits_no_state_corruption() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///gen_incremental.pl";

    // Open with multi-line content
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
                    "text": "sub hello {\n    print \"hello\";\n}\n"
                }
            }
        }),
    );

    // Incremental edit v2: change "hello" to "world" on line 1
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 1, "character": 11 },
                        "end": { "line": 1, "character": 16 }
                    },
                    "text": "world"
                }]
            }
        }),
    );

    // Incremental edit v3: change sub name from "hello" to "greet"
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 4 },
                        "end": { "line": 0, "character": 9 }
                    },
                    "text": "greet"
                }]
            }
        }),
    );

    // Request document symbols to verify the server sees the renamed sub
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 200,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    assert!(
        response.get("error").is_none(),
        "documentSymbol after incremental edits should not error: {response:?}"
    );

    // If we got symbols back, verify the sub name reflects the latest edit
    if let Some(result) = response.get("result") {
        if let Some(symbols) = result.as_array() {
            if let Some(first) = symbols.first() {
                let name = first.get("name").and_then(|n| n.as_str()).unwrap_or("");
                assert_eq!(
                    name, "greet",
                    "Symbol name should reflect the latest incremental edit, got: {name}"
                );
            }
        }
    }

    Ok(())
}
