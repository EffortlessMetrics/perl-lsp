mod common;

use common::{
    drain_until_quiet, initialize_lsp, send_notification, send_request, start_lsp_server,
};
use serde_json::json;
use std::time::Duration;

#[test]
fn malformed_range_did_change_does_not_crash_or_poison_followup_requests() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///malformed_range_regression.pl";
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
                    "text": "my $value = 1;\n"
                }
            }
        }),
    );

    // Reversed range is malformed per LSP ordering semantics.
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 10 },
                        "end": { "line": 0, "character": 2 }
                    },
                    "text": "BROKEN"
                }]
            }
        }),
    );
    drain_until_quiet(&server, Duration::from_millis(200), Duration::from_secs(3));

    let hover = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );

    assert!(hover.get("error").is_none(), "malformed didChange must not poison later requests");
    assert!(server.is_alive(), "server should remain alive after malformed didChange");
}

#[test]
fn multibyte_boundary_edit_can_fall_back_to_safe_full_replace() {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///multibyte_boundary_fallback.pl";
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
                    "text": "my $emoji = \"😀\";\n"
                }
            }
        }),
    );

    // Character 13 is inside the surrogate pair for 😀 in UTF-16.
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 13 },
                        "end": { "line": 0, "character": 14 }
                    },
                    "text": "X"
                }]
            }
        }),
    );

    // Full replacement on next version should keep the document in a known-good state.
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{
                    "text": "my $stable = 42;\n"
                }]
            }
        }),
    );
    drain_until_quiet(&server, Duration::from_millis(200), Duration::from_secs(3));

    let hover = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );

    assert!(hover.get("error").is_none(), "server should keep healthy state after fallback");
    assert!(server.is_alive(), "server should remain alive after multibyte boundary edit");
}
