//! End-to-end LSP smoke test over stdio using real JSON-RPC framing.

mod common;

use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn send_request_with_timeout(
    server: &mut common::LspServer,
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

#[test]
fn lsp_smoke_e2e_stdio_flow() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = common::start_lsp_server();
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
        &mut server,
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
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    common::send_notification(
        &mut server,
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
        &mut server,
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
        &mut server,
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
        &mut server,
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

    let shutdown_response =
        send_request_with_timeout(&mut server, 5, "shutdown", json!(null), timeout)?;
    assert!(
        shutdown_response.get("error").is_none(),
        "shutdown returned error: {shutdown_response:#}"
    );
    common::send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );

    let wait_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = server.process.try_wait()? {
            assert!(status.success(), "perl-lsp process exited with non-zero status: {status}");
            break;
        }

        if Instant::now() >= wait_deadline {
            let _ = server.process.kill();
            return Err("perl-lsp did not exit cleanly within timeout".into());
        }

        std::thread::sleep(Duration::from_millis(25));
    }

    Ok(())
}
