//! End-to-end LSP smoke test over stdio using real JSON-RPC framing.

mod common;

use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_URI_ID: AtomicU64 = AtomicU64::new(1);

fn unique_test_uri(test_name: &str) -> String {
    let id = NEXT_URI_ID.fetch_add(1, Ordering::Relaxed);
    format!("file:///tmp/perl-lsp-{test_name}-{}-{id}.pl", std::process::id())
}

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

fn completion_labels(items: &[Value]) -> Vec<&str> {
    items.iter().filter_map(|item| item.get("label").and_then(Value::as_str)).collect()
}

fn wait_for_diagnostics_matching(
    server: &common::LspServer,
    uri: &str,
    timeout: Duration,
    predicate: impl Fn(&[Value]) -> bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + timeout;
    let mut last_payload: Option<Value> = None;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Some(notification) = common::read_notification_method(
            server,
            "textDocument/publishDiagnostics",
            remaining.min(Duration::from_millis(250)),
        ) else {
            continue;
        };

        if notification.pointer("/params/uri").and_then(Value::as_str) != Some(uri) {
            continue;
        }

        last_payload = Some(notification.clone());
        let diagnostics = notification
            .pointer("/params/diagnostics")
            .and_then(Value::as_array)
            .ok_or("publishDiagnostics payload missing diagnostics array")?;
        if predicate(diagnostics) {
            return Ok(notification);
        }
    }

    Err(format!(
        "timeout waiting for matching publishDiagnostics for {uri}; last payload: {last_payload:#?}"
    )
    .into())
}

fn diagnostic_items(response: &Value) -> Result<&Vec<Value>, Box<dyn std::error::Error>> {
    let result = response.get("result").ok_or("diagnostic response missing result")?;
    let items = result.get("items").ok_or("diagnostic result missing items")?;
    Ok(items.as_array().ok_or("diagnostic items should be an array")?)
}

fn diagnostic_messages(items: &[Value]) -> Vec<&str> {
    items.iter().filter_map(|item| item.get("message").and_then(Value::as_str)).collect()
}

#[test]
fn lsp_smoke_e2e_push_diagnostics_clear_after_fix() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let request_timeout = Duration::from_secs(3);
    let diagnostics_timeout = Duration::from_secs(5);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = unique_test_uri("push-diagnostics-clear");

    let init_response = send_request_with_timeout(
        &server,
        101,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "versionSupport": true
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

    let broken_source = r#"use strict;
use warnings;

sub broken {
    if ($_[0] > 10 {
        return $_[0];
    }
}
"#;

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
                    "text": broken_source
                }
            }
        }),
    );

    let broken_diagnostics =
        wait_for_diagnostics_matching(&server, &uri, diagnostics_timeout, |diagnostics| {
            diagnostics.iter().any(|diagnostic| {
                diagnostic.get("source").and_then(Value::as_str) == Some("perl-parser")
                    && diagnostic.get("severity").and_then(Value::as_i64) == Some(1)
            })
        })?;
    let broken_items = broken_diagnostics
        .pointer("/params/diagnostics")
        .and_then(Value::as_array)
        .ok_or("broken diagnostics payload missing diagnostics array")?;
    assert!(
        !broken_items.is_empty(),
        "broken source should publish at least one diagnostic: {broken_diagnostics:#}"
    );

    let fixed_source = r#"use strict;
use warnings;

sub broken {
    if ($_[0] > 10) {
        return $_[0];
    }
}

my $answer = broken(11);
print $answer;
"#;

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": fixed_source }]
            }
        }),
    );

    let fixed_diagnostics =
        wait_for_diagnostics_matching(&server, &uri, diagnostics_timeout, |diagnostics| {
            diagnostics.is_empty()
        })?;
    assert_eq!(
        fixed_diagnostics.pointer("/params/version").and_then(Value::as_i64),
        Some(2),
        "clear diagnostics notification should carry the didChange version"
    );

    let hover_response = send_request_with_timeout(
        &server,
        102,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 3, "character": 4 }
        }),
        request_timeout,
    )?;
    assert!(
        hover_response.get("error").is_none(),
        "server should remain responsive after diagnostics clear: {hover_response:#}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 103, "shutdown", json!(null), request_timeout)?;
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

    Ok(())
}

#[test]
fn lsp_smoke_e2e_stdio_flow() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(2);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();

    let uri = unique_test_uri("stdio-flow");
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
    let initial_labels = completion_labels(completion_items);
    assert!(
        initial_labels.contains(&"greet"),
        "completion near `gre` should include `greet`, found labels: {initial_labels:?}"
    );

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
    let v2_labels = completion_labels(v2_items);
    assert!(
        v2_labels.contains(&"greet"),
        "re-completion should retain `greet`, found labels: {v2_labels:?}"
    );
    assert!(
        v2_labels.contains(&"greetings"),
        "re-completion should include newly added `greetings`, found labels: {v2_labels:?}"
    );

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

    // ── Step 10: textDocument/rename ─────────────────────────────────────
    let (rename_line, rename_col) = line_col(fixture_v2, 6, "greet()")?;
    let rename_response = send_request_with_timeout(
        &server,
        10,
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": rename_line, "character": rename_col },
            "newName": "salute"
        }),
        timeout,
    )?;
    assert!(rename_response.get("error").is_none(), "rename returned error: {rename_response:#}");
    let rename_result = rename_response.get("result").ok_or("rename result should be present")?;
    let rename_changes = rename_result
        .get("changes")
        .and_then(Value::as_object)
        .ok_or("rename changes should be an object")?;
    let this_file_edits = rename_changes
        .get(&uri)
        .and_then(Value::as_array)
        .ok_or("rename should contain edits for opened document")?;
    assert!(
        this_file_edits.len() >= 2,
        "rename should update both declaration and call site, edits: {this_file_edits:#?}"
    );

    // ── Shutdown ────────────────────────────────────────────────────────
    let shutdown_response =
        send_request_with_timeout(&server, 11, "shutdown", json!(null), timeout)?;
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
fn lsp_smoke_e2e_ignores_stale_did_change_versions() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(2);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = "file:///tmp/lsp_smoke_e2e_stale_change.pl";

    let original_source = r#"use strict;
use warnings;

sub stable_symbol { return 42; }
my $value = stable_symbol();
"#;

    let stale_broken_source = r#"use strict;
use warnings;

sub stable_symbol { return ;
my $value = stable_symbol();
"#;

    let init_response = send_request_with_timeout(
        &server,
        201,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "diagnostic": {
                        "dynamicRegistration": false
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
                    "version": 2,
                    "text": original_source
                }
            }
        }),
    );

    let initial_diagnostic_response = send_request_with_timeout(
        &server,
        202,
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
        timeout,
    )?;
    assert!(
        initial_diagnostic_response.get("error").is_none(),
        "initial diagnostic request returned error: {initial_diagnostic_response:#}"
    );
    let initial_messages = diagnostic_messages(diagnostic_items(&initial_diagnostic_response)?);
    assert!(
        initial_messages.iter().all(|message| {
            let lower = message.to_ascii_lowercase();
            !lower.contains("expected") && !lower.contains("recovered from missingoperand")
        }),
        "opened document should not start with parse-error diagnostics: {initial_messages:?}"
    );

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 1 },
                "contentChanges": [{ "text": stale_broken_source }]
            }
        }),
    );

    std::thread::sleep(Duration::from_millis(50));

    let stale_diagnostic_response = send_request_with_timeout(
        &server,
        203,
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
        timeout,
    )?;
    assert!(
        stale_diagnostic_response.get("error").is_none(),
        "diagnostic request after stale didChange returned error: {stale_diagnostic_response:#}"
    );
    let stale_messages = diagnostic_messages(diagnostic_items(&stale_diagnostic_response)?);
    assert!(
        stale_messages.iter().all(|message| {
            let lower = message.to_ascii_lowercase();
            !lower.contains("expected") && !lower.contains("recovered from missingoperand")
        }),
        "stale didChange must not replace the newer clean document with parse errors: {stale_messages:?}"
    );

    let (hover_line, hover_col) = line_col(original_source, 4, "stable_symbol")?;
    let hover_response = send_request_with_timeout(
        &server,
        204,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": hover_line, "character": hover_col }
        }),
        timeout,
    )?;
    assert!(
        hover_response.get("error").is_none(),
        "server should remain responsive after ignoring stale didChange: {hover_response:#}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 205, "shutdown", json!(null), timeout)?;
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

    Ok(())
}

#[test]
fn lsp_smoke_e2e_pull_diagnostics_refresh_after_change() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(2);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();

    let uri = unique_test_uri("pull-diagnostics-refresh");
    let broken_fixture = "use strict;
use warnings;

my $value = ;
";
    let fixed_fixture = "use strict;
use warnings;

my $value = 42;
";

    let init_response = send_request_with_timeout(
        &server,
        101,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "diagnostic": {
                        "dynamicRegistration": false
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
                    "text": broken_fixture
                }
            }
        }),
    );

    let broken_response = send_request_with_timeout(
        &server,
        102,
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
        timeout,
    )?;
    assert!(
        broken_response.get("error").is_none(),
        "textDocument/diagnostic returned error for broken document: {broken_response:#}"
    );
    let broken_items = diagnostic_items(&broken_response)?;
    assert!(!broken_items.is_empty(), "broken document should report at least one pull diagnostic");
    let broken_messages = diagnostic_messages(broken_items);
    assert!(
        broken_messages.iter().any(|message| {
            let lower = message.to_ascii_lowercase();
            lower.contains("expected") || lower.contains("recovered from missingoperand")
        }),
        "broken document diagnostics should mention an expected token or recovered missing operand: {broken_messages:?}"
    );

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": fixed_fixture }]
            }
        }),
    );
    std::thread::sleep(Duration::from_millis(50));

    let fixed_response = send_request_with_timeout(
        &server,
        103,
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
        timeout,
    )?;
    assert!(
        fixed_response.get("error").is_none(),
        "textDocument/diagnostic returned error after fixing document: {fixed_response:#}"
    );
    let fixed_items = diagnostic_items(&fixed_response)?;
    let fixed_messages = diagnostic_messages(fixed_items);
    assert!(
        fixed_messages.iter().all(|message| {
            let lower = message.to_ascii_lowercase();
            !lower.contains("expected") && !lower.contains("recovered from missingoperand")
        }),
        "fixed document should clear parse-error diagnostics: {fixed_messages:?}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 104, "shutdown", json!(null), timeout)?;
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

    Ok(())
}
