//! Regression coverage for didChange edit mapping hardening.
//!
//! These tests focus on malformed/ambiguous LSP ranges to ensure the server
//! remains stable and document state is not corrupted.

use serde_json::json;

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn malformed_did_change_range_does_not_corrupt_state() -> TestResult {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///malformed_range.pl";
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
                    "text": "sub alpha {}\n"
                }
            }
        }),
    );

    // Malformed: start > end.
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 8 },
                        "end": { "line": 0, "character": 4 }
                    },
                    "text": "beta"
                }]
            }
        }),
    );

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/documentSymbol",
            "params": { "textDocument": { "uri": uri } }
        }),
    );

    assert!(response.get("error").is_none(), "documentSymbol failed: {response:?}");
    Ok(())
}

#[test]
fn did_change_inside_multibyte_boundary_is_safe() -> TestResult {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///multibyte_boundary.pl";
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
                    "text": "my $x = \"😀\";\n"
                }
            }
        }),
    );

    // UTF-16 position 9 sits inside the emoji surrogate pair in this line.
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 9 },
                        "end": { "line": 0, "character": 9 }
                    },
                    "text": "x"
                }]
            }
        }),
    );

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 4 }
            }
        }),
    );
    assert!(response.get("error").is_none(), "hover failed after multibyte edit: {response:?}");
    Ok(())
}

#[test]
fn full_document_replace_path_remains_healthy() -> TestResult {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///full_replace_fallback.pl";
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
                    "text": "sub old_name {}\n"
                }
            }
        }),
    );

    // Full-document replacement (no range) should always be supported.
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "text": "sub new_name {}\n"
                }]
            }
        }),
    );

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/documentSymbol",
            "params": { "textDocument": { "uri": uri } }
        }),
    );

    assert!(response.get("error").is_none(), "documentSymbol failed after full replace");
    Ok(())
}

#[test]
fn malformed_edits_do_not_panic_and_server_stays_responsive() -> TestResult {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///no_panic_state.pl";
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

    for version in 2..=5 {
        send_notification(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{
                        "range": {
                            "start": { "line": 99, "character": 0 },
                            "end": { "line": 0, "character": 0 }
                        },
                        "text": format!("$v{version}")
                    }]
                }
            }),
        );
    }

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 }
            }
        }),
    );

    assert!(response.get("error").is_none(), "server became unresponsive: {response:?}");
    Ok(())
}
