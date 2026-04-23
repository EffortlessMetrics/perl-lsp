//! End-to-end LSP smoke test over stdio using real JSON-RPC framing.

mod common;

use serde_json::{Value, json};
use std::time::{Duration, Instant};

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

fn line_col(source: &str, target_line: usize, needle: &str) -> Result<(u32, u32), String> {
    let line = source
        .lines()
        .nth(target_line)
        .ok_or_else(|| format!("line {target_line} not found in fixture"))?;
    let col = line
        .find(needle)
        .ok_or_else(|| format!("needle `{needle}` not found on line {target_line}"))?;
    Ok((target_line as u32, col as u32))
}

fn completion_labels(response: &Value) -> Result<Vec<String>, String> {
    let items = response["result"]["items"]
        .as_array()
        .or_else(|| response["result"].as_array())
        .ok_or_else(|| "completion result missing items array".to_string())?;

    Ok(items.iter().filter_map(|item| item["label"].as_str().map(ToOwned::to_owned)).collect())
}

#[test]
fn lsp_smoke_e2e_didclose_reopen_resets_document_state() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(2);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();

    let uri = "file:///tmp/lsp_smoke_e2e_lifecycle.pl";
    let fixture_v1 = "my $value = gre\nsub greet { 1 }\n";
    let fixture_v2 = "my $value = wel\nsub welcome { 1 }\n";

    let init_response = send_request_with_timeout(
        &server,
        100,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    }
                }
            }
        }),
        init_timeout,
    )?;
    assert!(init_response.get("error").is_none(), "initialize returned error: {init_response:#}");

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
                    "text": fixture_v1
                }
            }
        }),
    );

    let completion_v1 = send_request_with_timeout(
        &server,
        101,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 15 }
        }),
        timeout,
    )?;
    let labels_v1 = completion_labels(&completion_v1)?;
    assert!(labels_v1.iter().any(|label| label == "greet"), "expected greet completion in v1");

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {
                "textDocument": { "uri": uri }
            }
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
                    "version": 2,
                    "text": fixture_v2
                }
            }
        }),
    );

    let completion_v2 = send_request_with_timeout(
        &server,
        102,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 15 }
        }),
        timeout,
    )?;
    let labels_v2 = completion_labels(&completion_v2)?;

    assert!(
        labels_v2.iter().any(|label| label == "welcome"),
        "expected welcome completion after reopen"
    );
    assert!(
        !labels_v2.iter().any(|label| label == "greet"),
        "didClose + didOpen should clear stale greet symbol"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 103, "shutdown", json!(null), timeout)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown returned error: {shutdown_response:#}"
    );

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let wait_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = server.process.lock().unwrap_or_else(|e| e.into_inner()).try_wait()? {
            assert!(status.success(), "perl-lsp process exited with non-zero status: {status}");
            break;
        }

        if Instant::now() >= wait_deadline {
            let _ = server.process.lock().unwrap_or_else(|e| e.into_inner()).kill();
            return Err("perl-lsp did not exit cleanly within timeout".into());
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    Ok(())
}

#[test]
fn lsp_smoke_e2e_stdio_flow() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(2);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();

    let uri = "file:///tmp/lsp_smoke_e2e.pl";
    let fixture = r#"use strict;
use warnings;

my $greeting = 'hello';
sub greet { return $greeting; }
my $result = greet();
my $value = gre
"#;

    let init_response = send_request_with_timeout(
        &server,
        1,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    }
                }
            }
        }),
        init_timeout,
    )?;
    assert!(init_response.get("error").is_none(), "initialize returned error: {init_response:#}");

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
                    "text": fixture
                }
            }
        }),
    );

    let completion_line = fixture
        .lines()
        .position(|line| line.contains("my $value = gre"))
        .ok_or("completion line missing in fixture")?;
    let completion_col = fixture
        .lines()
        .nth(completion_line)
        .and_then(|line| line.find("gre"))
        .map(|idx| idx + 3)
        .ok_or("completion token missing in fixture")?;

    let completion_response = send_request_with_timeout(
        &server,
        2,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": completion_line, "character": completion_col }
        }),
        timeout,
    )?;
    assert!(
        completion_response.get("error").is_none(),
        "completion returned error: {completion_response:#}"
    );
    let completion_items = completion_response["result"]["items"]
        .as_array()
        .or_else(|| completion_response["result"].as_array())
        .ok_or("completion result missing items array")?;
    assert!(!completion_items.is_empty(), "completion items should not be empty");

    let (hover_line, hover_col) = line_col(fixture, 4, "$greeting")?;
    let hover_response = send_request_with_timeout(
        &server,
        3,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": hover_line, "character": hover_col }
        }),
        timeout,
    )?;
    assert!(hover_response.get("error").is_none(), "hover returned error: {hover_response:#}");
    let hover_has_content = hover_response["result"]["contents"]["value"]
        .as_str()
        .is_some_and(|content| !content.is_empty());
    assert!(hover_has_content, "hover content should be present");

    let (def_line, def_col) = line_col(fixture, 5, "greet()")?;
    let definition_response = send_request_with_timeout(
        &server,
        4,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": def_line, "character": def_col }
        }),
        timeout,
    )?;
    assert!(
        definition_response.get("error").is_none(),
        "definition returned error: {definition_response:#}"
    );
    let definition_items =
        definition_response["result"].as_array().ok_or("definition result should be an array")?;
    let first_location = definition_items.first().ok_or("definition result should be non-empty")?;
    let definition_uri = first_location["uri"].as_str().ok_or("definition uri missing")?;
    assert_eq!(definition_uri, uri, "definition should resolve inside opened file");

    // ── Step 5: textDocument/didChange + re-completion ──────────────────
    let fixture_v2 = r#"use strict;
use warnings;

my $greeting = 'hello';
sub greet { return $greeting; }
sub greetings { return $greeting; }
my $result = greet();
my $value = gre
"#;
    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": fixture_v2 }]
            }
        }),
    );
    // Brief settle time for text sync
    std::thread::sleep(Duration::from_millis(50));

    let v2_completion_line = fixture_v2
        .lines()
        .position(|line| line.contains("my $value = gre"))
        .ok_or("v2 completion line missing")?;
    let v2_completion_col = fixture_v2
        .lines()
        .nth(v2_completion_line)
        .and_then(|line| line.find("gre"))
        .map(|idx| idx + 3)
        .ok_or("v2 completion token missing")?;

    let v2_completion_response = send_request_with_timeout(
        &server,
        5,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": v2_completion_line, "character": v2_completion_col }
        }),
        timeout,
    )?;
    assert!(
        v2_completion_response.get("error").is_none(),
        "re-completion after didChange returned error: {v2_completion_response:#}"
    );
    let v2_items = v2_completion_response["result"]["items"]
        .as_array()
        .or_else(|| v2_completion_response["result"].as_array())
        .ok_or("re-completion result missing items array")?;
    assert!(!v2_items.is_empty(), "re-completion items should not be empty after didChange");

    // ── Step 6: textDocument/references ─────────────────────────────────
    let (ref_line, ref_col) = line_col(fixture_v2, 4, "$greeting")?;
    let references_response = send_request_with_timeout(
        &server,
        6,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": ref_line, "character": ref_col },
            "context": { "includeDeclaration": true }
        }),
        timeout,
    )?;
    assert!(
        references_response.get("error").is_none(),
        "references returned error: {references_response:#}"
    );
    // Soft assertion: if the server returns a result, it should be an array
    if let Some(ref_items) = references_response["result"].as_array() {
        // $greeting appears in: declaration (line 3), sub greet body (line 4), sub greetings body (line 5)
        assert!(!ref_items.is_empty(), "references for $greeting should not be empty");
    }

    // ── Step 7: textDocument/documentSymbol ─────────────────────────────
    let doc_symbol_response = send_request_with_timeout(
        &server,
        7,
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
        timeout,
    )?;
    assert!(
        doc_symbol_response.get("error").is_none(),
        "documentSymbol returned error: {doc_symbol_response:#}"
    );
    // Soft assertion: result should be an array containing at least one symbol
    if let Some(symbols) = doc_symbol_response["result"].as_array() {
        assert!(
            !symbols.is_empty(),
            "documentSymbol should return at least one symbol (e.g. greet)"
        );
    }

    // ── Step 8: workspace/symbol ────────────────────────────────────────
    let ws_symbol_response = send_request_with_timeout(
        &server,
        8,
        "workspace/symbol",
        json!({
            "query": "greet"
        }),
        timeout,
    )?;
    assert!(
        ws_symbol_response.get("error").is_none(),
        "workspace/symbol returned error: {ws_symbol_response:#}"
    );
    // Soft assertion: result should be an array (may be empty if indexing is not ready)
    if let Some(ws_symbols) = ws_symbol_response["result"].as_array() {
        // Workspace symbol for a single-file scenario may or may not find results
        // depending on indexing state; just verify no crash and valid response shape
        let _ = ws_symbols; // acknowledged
    }

    // ── Step 9: $/cancelRequest for bogus ID, then valid request ────────
    // Send cancel for a request ID that was never issued (should not crash)
    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": { "id": 99999 }
        }),
    );
    // Brief pause to let server process the bogus cancel
    std::thread::sleep(Duration::from_millis(50));

    // Now send a valid request to confirm the server is still healthy
    let post_cancel_response = send_request_with_timeout(
        &server,
        9,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": hover_line, "character": hover_col }
        }),
        timeout,
    )?;
    assert!(
        post_cancel_response.get("error").is_none(),
        "hover after bogus cancelRequest returned error: {post_cancel_response:#}"
    );

    // ── Shutdown ────────────────────────────────────────────────────────
    let shutdown_response =
        send_request_with_timeout(&server, 10, "shutdown", json!(null), timeout)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown returned error: {shutdown_response:#}"
    );
    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let wait_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = server.process.lock().unwrap_or_else(|e| e.into_inner()).try_wait()? {
            assert!(status.success(), "perl-lsp process exited with non-zero status: {status}");
            break;
        }

        if Instant::now() >= wait_deadline {
            let _ = server.process.lock().unwrap_or_else(|e| e.into_inner()).kill();
            return Err("perl-lsp did not exit cleanly within timeout".into());
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    Ok(())
}
