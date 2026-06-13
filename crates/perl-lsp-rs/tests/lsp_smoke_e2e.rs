//! End-to-end LSP smoke test over stdio using real JSON-RPC framing.

mod common;

use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
fn lsp_smoke_e2e_did_close_clears_published_diagnostics() -> TestResult {
    let server = common::start_lsp_server();
    let request_timeout = Duration::from_secs(3);
    let diagnostics_timeout = Duration::from_secs(5);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = unique_test_uri("didclose-diagnostics");

    let init_response = send_request_with_timeout(
        &server,
        201,
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

sub close_me {
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
        "broken source should publish at least one diagnostic before close: {broken_diagnostics:#}"
    );

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

    let clear_diagnostics =
        wait_for_diagnostics_matching(&server, &uri, diagnostics_timeout, |diagnostics| {
            diagnostics.is_empty()
        })?;
    let clear_items = clear_diagnostics
        .pointer("/params/diagnostics")
        .and_then(Value::as_array)
        .ok_or("clear diagnostics payload missing diagnostics array")?;
    assert!(
        clear_items.is_empty(),
        "didClose should publish an empty diagnostics array to clear stale editor diagnostics: {clear_diagnostics:#}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 202, "shutdown", json!(null), request_timeout)?;
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
    let timeout = common::timeout_scaler::TimeoutProfile::Standard.timeout();
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
        common::timeout_scaler::TimeoutProfile::CrossFile.timeout(),
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
fn lsp_smoke_e2e_reopen_same_uri_replaces_document_symbols() -> TestResult {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(2);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();

    let uri = unique_test_uri("reopen-lifecycle");
    let first_fixture = r#"use strict;
use warnings;

sub greet { return 'hello'; }
my $value = gre
"#;
    let second_fixture = r#"use strict;
use warnings;

sub goodbye { return 'bye'; }
my $value = goo
"#;

    let init_response = send_request_with_timeout(
        &server,
        206,
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
                    "text": first_fixture
                }
            }
        }),
    );

    let first_completion_line = first_fixture
        .lines()
        .position(|line| line.contains("my $value = gre"))
        .ok_or("first completion line missing in fixture")?;
    let first_completion_col = first_fixture
        .lines()
        .nth(first_completion_line)
        .and_then(|line| line.find("gre"))
        .map(|idx| idx + 3)
        .ok_or("first completion token missing in fixture")?;
    let first_completion_response = send_request_with_timeout(
        &server,
        207,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": first_completion_line, "character": first_completion_col }
        }),
        timeout,
    )?;
    assert!(
        first_completion_response.get("error").is_none(),
        "initial completion returned error: {first_completion_response:#}"
    );
    let first_completion_items = first_completion_response["result"]["items"]
        .as_array()
        .or_else(|| first_completion_response["result"].as_array())
        .ok_or("initial completion result missing items array")?;
    let first_labels = completion_labels(first_completion_items);
    assert!(
        first_labels.contains(&"greet"),
        "initial completion should include symbol from first open, found labels: {first_labels:?}"
    );

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
                    "version": 1,
                    "text": second_fixture
                }
            }
        }),
    );
    std::thread::sleep(Duration::from_millis(50));

    let second_completion_line = second_fixture
        .lines()
        .position(|line| line.contains("my $value = goo"))
        .ok_or("second completion line missing in fixture")?;
    let second_completion_col = second_fixture
        .lines()
        .nth(second_completion_line)
        .and_then(|line| line.find("goo"))
        .map(|idx| idx + 3)
        .ok_or("second completion token missing in fixture")?;
    let second_completion_response = send_request_with_timeout(
        &server,
        208,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": second_completion_line, "character": second_completion_col }
        }),
        timeout,
    )?;
    assert!(
        second_completion_response.get("error").is_none(),
        "completion after reopen returned error: {second_completion_response:#}"
    );
    let second_completion_items = second_completion_response["result"]["items"]
        .as_array()
        .or_else(|| second_completion_response["result"].as_array())
        .ok_or("reopened completion result missing items array")?;
    let second_labels = completion_labels(second_completion_items);
    assert!(
        second_labels.contains(&"goodbye"),
        "completion after reopen should include symbol from replacement document, found labels: {second_labels:?}"
    );
    assert!(
        !second_labels.contains(&"greet"),
        "completion after didClose + reopen must not leak stale symbols from the prior document: {second_labels:?}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 209, "shutdown", json!(null), timeout)?;
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
fn lsp_smoke_e2e_document_intelligence_shapes() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(3);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = unique_test_uri("document-intelligence");

    let fixture = r#"package Smoke::DocumentIntelligence;
use strict;
use warnings;

sub compute_total {
    my ($limit) = @_;
    my $total = 0;
    for my $idx (1 .. $limit) {
        if ($idx % 2 == 0) {
            $total += $idx;
        }
    }
    return $total;
}

my $answer = compute_total(10);
print $answer;
"#;

    let init_response = send_request_with_timeout(
        &server,
        210,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "documentHighlight": { "dynamicRegistration": false },
                    "foldingRange": { "dynamicRegistration": false },
                    "selectionRange": { "dynamicRegistration": false },
                    "semanticTokens": {
                        "dynamicRegistration": false,
                        "requests": { "full": true },
                        "tokenTypes": [
                            "namespace", "type", "class", "interface", "enum", "enumMember",
                            "typeParameter", "function", "method", "property", "macro", "variable",
                            "parameter", "keyword", "modifier", "comment", "string", "number",
                            "regexp", "operator"
                        ],
                        "tokenModifiers": []
                    },
                    "inlayHint": { "dynamicRegistration": false }
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

    let has_lsp_range = |range: &Value| -> bool {
        range.pointer("/start/line").and_then(Value::as_u64).is_some()
            && range.pointer("/start/character").and_then(Value::as_u64).is_some()
            && range.pointer("/end/line").and_then(Value::as_u64).is_some()
            && range.pointer("/end/character").and_then(Value::as_u64).is_some()
    };

    let (compute_line, compute_col) = line_col(fixture, 4, "compute_total")?;
    let highlight_response = send_request_with_timeout(
        &server,
        211,
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": compute_line, "character": compute_col }
        }),
        timeout,
    )?;
    assert!(
        highlight_response.get("error").is_none(),
        "documentHighlight returned error: {highlight_response:#}"
    );
    if let Some(highlights) = highlight_response["result"].as_array() {
        assert!(
            highlights.iter().all(|item| item.get("range").is_some_and(has_lsp_range)),
            "documentHighlight entries should include valid ranges: {highlight_response:#}"
        );
    } else {
        assert!(
            highlight_response["result"].is_null(),
            "documentHighlight result should be an array or null: {highlight_response:#}"
        );
    }

    let folding_response = send_request_with_timeout(
        &server,
        212,
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        timeout,
    )?;
    assert!(
        folding_response.get("error").is_none(),
        "foldingRange returned error: {folding_response:#}"
    );
    let folding_ranges =
        folding_response["result"].as_array().ok_or("foldingRange result should be an array")?;
    assert!(
        folding_ranges.iter().any(|range| {
            range.get("startLine").and_then(Value::as_u64).is_some()
                && range.get("endLine").and_then(Value::as_u64).is_some()
        }),
        "foldingRange should return at least one range with line fields: {folding_response:#}"
    );

    let selection_response = send_request_with_timeout(
        &server,
        213,
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": uri },
            "positions": [
                { "line": compute_line, "character": compute_col },
                { "line": 9, "character": 13 }
            ]
        }),
        timeout,
    )?;
    assert!(
        selection_response.get("error").is_none(),
        "selectionRange returned error: {selection_response:#}"
    );
    let selection_ranges = selection_response["result"]
        .as_array()
        .ok_or("selectionRange result should be an array")?;
    assert_eq!(
        selection_ranges.len(),
        2,
        "selectionRange should return one root range per requested position"
    );
    for selection_range in selection_ranges {
        assert!(
            selection_range.get("range").is_some_and(has_lsp_range),
            "selectionRange root entries should include valid ranges: {selection_response:#}"
        );
        if let Some(parent) = selection_range.get("parent") {
            assert!(
                parent.is_object() && parent.get("range").is_some_and(has_lsp_range),
                "selectionRange parents should preserve nested range shape: {selection_response:#}"
            );
        }
    }

    let semantic_response = send_request_with_timeout(
        &server,
        214,
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": uri } }),
        timeout,
    )?;
    assert!(
        semantic_response.get("error").is_none(),
        "semanticTokens/full returned error: {semantic_response:#}"
    );
    let semantic_data = semantic_response
        .pointer("/result/data")
        .and_then(Value::as_array)
        .ok_or("semanticTokens/full result should include a data array")?;
    assert!(
        !semantic_data.is_empty(),
        "semanticTokens/full should emit tokens for a non-empty Perl file"
    );
    assert_eq!(
        semantic_data.len() % 5,
        0,
        "semanticTokens/full data should be encoded in five-integer chunks"
    );

    let print_line_index =
        fixture.lines().position(|line| line == "print $answer;").ok_or("print line missing")?;
    let print_line_len = fixture.lines().nth(print_line_index).ok_or("print line missing")?.len();
    let inlay_response = send_request_with_timeout(
        &server,
        215,
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": print_line_index, "character": print_line_len }
            }
        }),
        timeout,
    )?;
    assert!(inlay_response.get("error").is_none(), "inlayHint returned error: {inlay_response:#}");
    if let Some(inlay_hints) = inlay_response["result"].as_array() {
        assert!(
            inlay_hints
                .iter()
                .all(|item| item.get("position").is_some() && item.get("label").is_some()),
            "inlayHint entries should include position and label fields: {inlay_response:#}"
        );
    } else {
        assert!(
            inlay_response["result"].is_null(),
            "inlayHint result should be an array or null: {inlay_response:#}"
        );
    }

    let shutdown_response =
        send_request_with_timeout(&server, 216, "shutdown", json!(null), timeout)?;
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

#[test]
fn lsp_smoke_e2e_will_save_wait_until_request_response() -> Result<(), Box<dyn std::error::Error>> {
    let server = common::start_lsp_server();
    // willSaveWaitUntil runs the on-save formatter, which shells out to perltidy
    // via an OsSubprocessRuntime with a 10s subprocess timeout. On runners where
    // perltidy is present (e.g. CI's perl-equipped CX lane) a cold/loaded spawn can
    // take several seconds, so the client timeout must comfortably exceed the
    // server-side 10s formatter timeout — otherwise the request times out before the
    // server responds. (Runners without perltidy return [] near-instantly.)
    let request_timeout = Duration::from_secs(15);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();

    let uri = "file:///tmp/lsp_smoke_e2e_will_save_wait_until.pl";
    let source = "use strict;\nuse warnings;\n\nmy $x=42;my $y=99;\nsub foo{return 1;}\n";

    // ── Step 1: Initialize ──────────────────────────────────────────────
    let init_response_result = send_request_with_timeout(
        &server,
        101,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "synchronization": {
                        "willSaveWaitUntil": true
                    }
                }
            }
        }),
        init_timeout,
    );
    assert!(
        init_response_result.is_ok(),
        "initialize response should arrive before timeout: {init_response_result:#?}"
    );
    let init_response = init_response_result?;
    assert!(init_response.get("error").is_none(), "initialize returned error: {init_response:#}");

    // Gap 2 fix: assert server advertises willSaveWaitUntil capability.
    // Capability lives at textDocumentSync.willSaveWaitUntil, NOT at top-level
    // willSaveWaitUntilProvider.
    assert_eq!(
        init_response.pointer("/result/capabilities/textDocumentSync/willSaveWaitUntil"),
        Some(&serde_json::Value::Bool(true)),
        "server must advertise textDocumentSync.willSaveWaitUntil = true in capabilities"
    );

    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    // ── Step 2: Open document ───────────────────────────────────────────
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
                    "text": source
                }
            }
        }),
    );

    // ── Step 3: Send willSaveWaitUntil request ──────────────────────────
    let will_save_response_result = send_request_with_timeout(
        &server,
        102,
        "textDocument/willSaveWaitUntil",
        json!({
            "textDocument": { "uri": uri },
            "reason": 1  // TextDocumentSaveReason.Manual
        }),
        request_timeout,
    );
    let will_save_response_status = if will_save_response_result.is_ok() { "ok" } else { "error" };
    assert_eq!(
        will_save_response_status, "ok",
        "willSaveWaitUntil response should arrive before timeout: {will_save_response_result:#?}"
    );
    let will_save_response = will_save_response_result?;

    // ── Step 4: Verify response envelope ────────────────────────────────
    assert!(
        will_save_response.get("error").is_none(),
        "willSaveWaitUntil returned error: {will_save_response:#}"
    );

    let result = will_save_response
        .get("result")
        .ok_or("willSaveWaitUntil result field should be present")?;

    // Gap 1 fix: LSP spec allows TextEdit[] | null; treat null as no-edits.
    // This implementation always returns an array, but be robust to spec-compliant nulls.
    assert!(
        matches!(result, serde_json::Value::Null | serde_json::Value::Array(_)),
        "willSaveWaitUntil result should be TextEdit[] or null, got: {result}"
    );
    let edits: &[serde_json::Value] = match result {
        serde_json::Value::Null => &[],
        serde_json::Value::Array(arr) => arr.as_slice(),
        _ => &[],
    };

    // If server returns edits, validate they have the required TextEdit structure.
    // Under default config (test_runner_enabled: true) the formatter runs, so edits
    // are likely non-empty for this fixture — the loop will execute.
    for edit in edits {
        let range = edit.get("range").ok_or("TextEdit should have range field")?;
        let _new_text = edit
            .get("newText")
            .and_then(|v| v.as_str())
            .ok_or("TextEdit should have newText string field")?;

        let start = range.get("start").ok_or("TextEdit range should have start")?;
        assert!(
            start.get("line").and_then(|v| v.as_u64()).is_some(),
            "TextEdit range.start.line should be a non-negative integer"
        );
        assert!(
            start.get("character").and_then(|v| v.as_u64()).is_some(),
            "TextEdit range.start.character should be a non-negative integer"
        );

        let end = range.get("end").ok_or("TextEdit range should have end")?;
        assert!(
            end.get("line").and_then(|v| v.as_u64()).is_some(),
            "TextEdit range.end.line should be a non-negative integer"
        );
        assert!(
            end.get("character").and_then(|v| v.as_u64()).is_some(),
            "TextEdit range.end.character should be a non-negative integer"
        );
    }

    // ── Step 5: Verify server is still responsive ────────────────────────
    let hover_response_result = send_request_with_timeout(
        &server,
        103,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 3, "character": 5 }
        }),
        request_timeout,
    );
    let hover_response_status = if hover_response_result.is_ok() { "ok" } else { "error" };
    assert_eq!(
        hover_response_status, "ok",
        "hover response should arrive after willSaveWaitUntil: {hover_response_result:#?}"
    );
    let hover_response = hover_response_result?;
    assert!(
        hover_response.get("error").is_none(),
        "server should remain responsive after willSaveWaitUntil: {hover_response:#}"
    );

    // ── Step 6: Send willSave notification ──────────────────────────────
    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/willSave",
            "params": {
                "textDocument": { "uri": uri },
                "reason": 1
            }
        }),
    );

    // ── Step 7: Send didSave to complete lifecycle ──────────────────────
    // Gap 3 fix: DidSaveTextDocumentParams.textDocument is TextDocumentIdentifier
    // (uri only — no version field).
    common::send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    // ── Step 8: Shutdown ────────────────────────────────────────────────
    let shutdown_response =
        send_request_with_timeout(&server, 104, "shutdown", json!(null), request_timeout)?;
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
fn lsp_smoke_e2e_code_action_envelope() -> TestResult {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(3);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = unique_test_uri("code-action");

    // No strict/warnings → BuiltIn analyzer fires; unqualified global $x triggers actions.
    let fixture = "package Smoke::CodeActions;\n$x = 1;\nsub calculate { return $x + 1; }\n";

    let init_response = send_request_with_timeout(
        &server,
        301,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }),
        init_timeout,
    )?;
    assert!(init_response.get("error").is_none(), "initialize returned error: {init_response:#}");

    common::send_notification(
        &server,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
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

    let line_count = fixture.lines().count() as u64;
    let code_action_response = send_request_with_timeout(
        &server,
        302,
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": line_count, "character": 0 }
            },
            "context": { "diagnostics": [] }
        }),
        timeout,
    )?;
    assert!(
        code_action_response.get("error").is_none(),
        "codeAction returned error: {code_action_response:#}"
    );
    let actions = code_action_response["result"].as_array().ok_or(
        "codeAction result should be an array (LSP spec: CodeAction[] | Command[] | null)",
    )?;
    assert!(
        actions.iter().any(|a| a.get("title").and_then(Value::as_str).is_some()),
        "codeAction response should include at least one action with a title: {code_action_response:#}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 303, "shutdown", json!(null), timeout)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown returned error: {shutdown_response:#}"
    );
    common::send_notification(
        &server,
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );

    Ok(())
}

#[test]
fn lsp_smoke_e2e_inline_completion_envelope() -> TestResult {
    let server = common::start_lsp_server();
    let timeout = Duration::from_secs(3);
    let init_timeout = common::timeout_scaler::TimeoutProfile::Initialization.timeout();
    let uri = unique_test_uri("inline-completion");

    // `use ` triggers the deterministic module-name provider; strict/warnings are always present.
    let fixture = "use ";

    // Advertise inlineCompletion without dynamicRegistration so the server includes
    // inlineCompletionProvider in its static capabilities response.
    let init_response = send_request_with_timeout(
        &server,
        401,
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {
                "textDocument": {
                    "inlineCompletion": {}
                }
            }
        }),
        init_timeout,
    )?;
    assert!(init_response.get("error").is_none(), "initialize returned error: {init_response:#}");
    assert!(
        init_response.pointer("/result/capabilities/inlineCompletionProvider").is_some(),
        "server must advertise inlineCompletionProvider when client requests it: {init_response:#}"
    );

    common::send_notification(
        &server,
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
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

    let inline_response = send_request_with_timeout(
        &server,
        402,
        "textDocument/inlineCompletion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 },
            "context": { "triggerKind": 1 }
        }),
        timeout,
    )?;
    assert!(
        inline_response.get("error").is_none(),
        "inlineCompletion returned error: {inline_response:#}"
    );
    let items = inline_response
        .pointer("/result/items")
        .and_then(Value::as_array)
        .ok_or("inlineCompletion result should be { items: [...] }")?;
    assert!(
        !items.is_empty(),
        "inlineCompletion at `use ` should return module suggestions: {inline_response:#}"
    );
    assert!(
        items.iter().any(|item| item.get("insertText").and_then(Value::as_str).is_some()),
        "inlineCompletion items should each have an insertText string field: {inline_response:#}"
    );

    // Verify `strict;` and `warnings;` are among the suggestions (deterministic, always present).
    let insert_texts: Vec<&str> =
        items.iter().filter_map(|item| item.get("insertText").and_then(Value::as_str)).collect();
    assert!(
        insert_texts.contains(&"strict;"),
        "`strict;` should be a suggested completion after `use `: {inline_response:#}"
    );
    assert!(
        insert_texts.contains(&"warnings;"),
        "`warnings;` should be a suggested completion after `use `: {inline_response:#}"
    );

    // Verify envelope shape — result must be an object, not a bare array.
    assert!(
        inline_response.pointer("/result").is_some_and(Value::is_object),
        "inlineCompletion result envelope must be an object {{ items: [...] }}: {inline_response:#}"
    );

    let shutdown_response =
        send_request_with_timeout(&server, 403, "shutdown", json!(null), timeout)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown returned error: {shutdown_response:#}"
    );
    common::send_notification(
        &server,
        json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
    );

    Ok(())
}
