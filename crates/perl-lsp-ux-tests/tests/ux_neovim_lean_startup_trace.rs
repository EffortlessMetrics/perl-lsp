//! Startup trace receipt for the Neovim lean profile.
//!
//! This is an e2e wiring receipt, not a hard latency budget. It records the
//! observed lean startup path and asserts that the no-eager-indexing and
//! no-file-watcher dials are active.

use anyhow::Result;
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness, binary_available};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const TRACE_SOURCE: &str = r#"use strict;
use warnings;

my $value = 42;
my $other = $val
sub broken {
"#;

fn trace_config(timeout: Duration) -> ScenarioConfig {
    ScenarioConfig {
        timeout,
        path_restriction: None,
        echo_stderr: false,
        extra_env: vec![
            ("PERL_LSP_E2E".to_string(), Some("1".to_string())),
            ("PERL_LSP_DIAGNOSTIC_MODE".to_string(), Some("syntax-only".to_string())),
            ("PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS".to_string(), Some("0".to_string())),
            ("PERL_LSP_EAGER_WORKSPACE_INDEXING".to_string(), Some("false".to_string())),
            ("PERL_LSP_FILE_WATCHERS".to_string(), Some("false".to_string())),
            ("PERL_LSP_QUIET".to_string(), Some("1".to_string())),
            (
                "RUST_LOG".to_string(),
                Some("perl_lsp::runtime::dispatch::lifecycle=debug".to_string()),
            ),
        ],
        workspace_files: Vec::new(),
        workspace_folders: vec![("project".to_string(), "trace-project".to_string())],
        client_capability_overrides: json!({
            "workspace": {
                "didChangeWatchedFiles": {
                    "dynamicRegistration": true
                }
            },
            "textDocument": {
                "inlineCompletion": {
                    "dynamicRegistration": true
                },
                "semanticTokens": {
                    "requests": {
                        "full": true,
                        "range": true
                    }
                }
            }
        }),
        initialization_options: Value::Null,
    }
}

fn record_event(events: &mut Vec<Value>, name: &str, start: Instant) {
    events.push(json!({
        "name": name,
        "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0,
    }));
}

fn wait_for_stderr_line(harness: &UxHarness, needle: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if harness.client.peek_stderr_lines().iter().any(|line| line.contains(needle)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

fn file_watcher_registered(events: &[LspEvent]) -> bool {
    registration_seen(events, "workspace/didChangeWatchedFiles")
}

fn registration_seen(events: &[LspEvent], method_name: &str) -> bool {
    events.iter().any(|event| {
        let LspEvent::Other { method, params } = event else {
            return false;
        };
        method == "client/registerCapability"
            && params.get("registrations").and_then(Value::as_array).into_iter().flatten().any(
                |registration| {
                    registration.get("method").and_then(Value::as_str) == Some(method_name)
                },
            )
    })
}

fn wait_for_registration(harness: &UxHarness, method_name: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if registration_seen(&harness.client.peek_events(), method_name) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

#[test]
fn ux_neovim_lean_startup_trace_receipt() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP ux_neovim_lean_startup_trace_receipt: perl-lsp binary not found");
        return Ok(());
    }

    let start = Instant::now();
    let mut events = Vec::new();
    record_event(&mut events, "process_start_observed", start);

    let harness = UxHarness::new(trace_config(Duration::from_secs(8)))?;
    record_event(&mut events, "initialize_response_received", start);
    record_event(&mut events, "initialized_notification_sent", start);

    let init = harness.client.initialize_result();
    assert_eq!(
        init.pointer("/result/capabilities/semanticTokensProvider/full"),
        Some(&json!(true)),
        "lean startup trace must not advertise semantic token delta support"
    );
    record_event(&mut events, "semantic_tokens_capability_checked", start);

    assert_eq!(
        init.pointer("/result/capabilities/workspace/textDocumentContent/schemes"),
        Some(&json!(["perldoc"])),
        "lean startup trace must keep perldoc textDocumentContent capability advertised"
    );
    record_event(&mut events, "text_document_content_capability_checked", start);

    let inline_registered =
        wait_for_registration(&harness, "textDocument/inlineCompletion", Duration::from_secs(2));
    assert!(
        inline_registered,
        "lean startup trace must dynamically register inline completion for LSP4IJ-shaped clients"
    );
    record_event(&mut events, "inline_completion_registration_checked", start);

    let indexing_skip_observed = wait_for_stderr_line(
        &harness,
        "Skipping eager workspace indexing on `initialized`",
        Duration::from_secs(2),
    );
    assert!(
        indexing_skip_observed,
        "lean startup trace must observe eager workspace indexing skipped; stderr={:?}",
        harness.client.peek_stderr_lines()
    );
    record_event(&mut events, "workspace_indexing_decision_observed", start);

    let watcher_registered = file_watcher_registered(&harness.client.peek_events());
    assert!(!watcher_registered, "lean startup trace must not register workspace file watchers");
    record_event(&mut events, "file_watcher_registration_checked", start);

    harness.open_file("project/trace.pl", TRACE_SOURCE)?;
    record_event(&mut events, "did_open_sent", start);

    let diags = harness.wait_for_diagnostics("project/trace.pl", Duration::from_secs(5));
    assert!(!diags.is_empty(), "syntax-only lean trace must publish parser diagnostics");
    record_event(&mut events, "first_did_open_processed", start);
    record_event(&mut events, "first_diagnostic_published", start);

    let _items = harness.completion("project/trace.pl", 4, 16)?;
    record_event(&mut events, "first_completion_response", start);

    let receipt = json!({
        "profile": "neovim_lean",
        "workspace_indexing_started": false,
        "workspace_indexing_decision_observed": indexing_skip_observed,
        "file_watchers_registered": watcher_registered,
        "inline_completion_registered": inline_registered,
        "semantic_tokens_delta_advertised": false,
        "text_document_content_schemes": ["perldoc"],
        "diagnostic_mode": "syntax_only",
        "diagnostic_debounce_ms": 0,
        "events": events,
    });
    println!("{}", serde_json::to_string_pretty(&receipt)?);

    harness.assert_no_crash();
    Ok(())
}
