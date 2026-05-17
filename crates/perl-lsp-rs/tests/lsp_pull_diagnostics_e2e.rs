//! End-to-end pull diagnostics coverage over stdio.
//!
//! This test exercises the real `perl-lsp` binary, JSON-RPC framing, text sync,
//! and `textDocument/diagnostic` together so regressions cannot hide behind the
//! in-process harness.

mod common;

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

fn send_request_with_timeout(
    server: &common::LspServer,
    id: i64,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, Box<dyn std::error::Error>> {
    common::send_request_no_wait(
        server,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }),
    );

    match common::read_response_matching_i64(server, id, timeout) {
        Some(response) => Ok(response),
        None => Err(format!("timeout waiting for response id={id} method={method}").into()),
    }
}

fn diagnostic_codes(response: &Value) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let items = response
        .get("result")
        .and_then(|result| result.get("items"))
        .and_then(Value::as_array)
        .ok_or("diagnostic response should contain result.items array")?;

    Ok(items
        .iter()
        .filter_map(|item| item.get("code").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

#[test]
fn pull_diagnostics_e2e_tracks_text_sync_fixes() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = "file:///tmp/lsp_pull_diagnostics_e2e.pl";
    let missing_pragmas = "my $value = 42;\nprint $value;\n";
    let fixed_pragmas = "use strict;\nuse warnings;\nmy $value = 42;\nprint $value;\n";

    let initialize = send_request_with_timeout(
        &server,
        1,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "diagnostic": {
                        "dynamicRegistration": false,
                        "relatedDocumentSupport": true
                    }
                }
            }
        }),
        timeout,
    )?;
    assert!(initialize.get("error").is_none(), "initialize returned error: {initialize:#}");

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": missing_pragmas
                }
            }
        }),
    );

    let first_diagnostics = send_request_with_timeout(
        &server,
        2,
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri },
            "identifier": "perl-lsp",
            "previousResultId": null
        }),
        timeout,
    )?;
    assert!(
        first_diagnostics.get("error").is_none(),
        "initial pull diagnostics returned error: {first_diagnostics:#}"
    );
    let first_codes = diagnostic_codes(&first_diagnostics)?;
    assert!(first_codes.contains("PL100"), "missing strict should report PL100: {first_codes:?}");
    assert!(first_codes.contains("PL101"), "missing warnings should report PL101: {first_codes:?}");

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": fixed_pragmas }]
            }
        }),
    );

    let second_diagnostics = send_request_with_timeout(
        &server,
        3,
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri },
            "identifier": "perl-lsp",
            "previousResultId": first_diagnostics["result"]["resultId"]
        }),
        timeout,
    )?;
    assert!(
        second_diagnostics.get("error").is_none(),
        "pull diagnostics after didChange returned error: {second_diagnostics:#}"
    );
    let second_codes = diagnostic_codes(&second_diagnostics)?;
    assert!(
        !second_codes.contains("PL100"),
        "adding use strict should clear PL100: {second_codes:?}"
    );
    assert!(
        !second_codes.contains("PL101"),
        "adding use warnings should clear PL101: {second_codes:?}"
    );

    let shutdown = send_request_with_timeout(&server, 4, "shutdown", json!(null), timeout)?;
    assert!(shutdown.get("error").is_none(), "shutdown returned error: {shutdown:#}");
    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    Ok(())
}
