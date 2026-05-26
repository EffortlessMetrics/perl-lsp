//! Raw-RPC latency receipts for the post-cutover Neovim lean-mode lane.
//!
//! Five scenarios that exercise the e2e runtime path against the real LSP
//! binary. They prove that:
//!
//! 1. `open -> completion` returns a useful answer (any non-error response).
//! 2. `open -> hover` returns a useful answer.
//! 3. `edit -> parse-error diagnostic` surfaces a parse error.
//! 4. `edit -> diagnostic clear` clears diagnostics when the parse cleans.
//! 5. `rapid typing -> latest completion wins` - under a burst of `didChange`
//!    notifications, the last completion request still returns successfully.
//! 6. Dynamic `textDocument/inlineCompletion` returns deterministic items while
//!    watcher registration stays off in the lean profile.
//!
//! These are intentionally "does it work end-to-end" tests, not numeric
//! latency assertions. CI machine variance makes wallclock budgets
//! brittle; the receipt is "we drove the e2e config and the answer
//! arrived." Wallclock measurements belong on dedicated benchmark
//! hardware, not in `cargo test`.
//!
//! Run with:
//!
//!     PERL_LSP_E2E=1 \
//!     PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS=0 \
//!     PERL_LSP_DIAGNOSTIC_MODE=syntax-only \
//!     PERL_LSP_EAGER_WORKSPACE_INDEXING=false \
//!     PERL_LSP_FILE_WATCHERS=false \
//!     cargo test -p perl-lsp-ux-tests --test ux_latency_raw_rpc \
//!         -- --test-threads=1 --nocapture

use anyhow::Result;
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness, binary_available};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SHORT_SOURCE: &str = r#"use strict;
use warnings;

my $value = 42;
my $other = $val
"#;

const PARSE_ERROR_SOURCE: &str = r#"use strict;
use warnings;

sub broken {
"#;

const CLEAN_SOURCE: &str = r#"use strict;
use warnings;

sub broken {}
"#;

/// Build an e2e harness config: syntax-only diagnostics, zero debounce, no
/// eager workspace indexing, no file watchers. These values mirror
/// `perllsp --runtime-mode e2e`, but are set explicitly so the receipt
/// documents the exact lean runtime path it exercises.
fn e2e_config(timeout: Duration) -> ScenarioConfig {
    ScenarioConfig {
        timeout,
        path_restriction: None,
        echo_stderr: false,
        extra_env: vec![
            ("PERL_LSP_E2E".to_string(), Some("1".to_string())),
            ("PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS".to_string(), Some("0".to_string())),
            ("PERL_LSP_DIAGNOSTIC_MODE".to_string(), Some("syntax-only".to_string())),
            ("PERL_LSP_EAGER_WORKSPACE_INDEXING".to_string(), Some("false".to_string())),
            ("PERL_LSP_FILE_WATCHERS".to_string(), Some("false".to_string())),
            // Quiet the startup banner so test output is uncluttered.
            ("PERL_LSP_QUIET".to_string(), Some("1".to_string())),
        ],
        workspace_files: Vec::new(),
        workspace_folders: Vec::new(),
        client_capability_overrides: json!({
            "workspace": {
                "didChangeWatchedFiles": {
                    "dynamicRegistration": true
                }
            },
            "textDocument": {
                "inlineCompletion": {
                    "dynamicRegistration": true
                }
            }
        }),
        initialization_options: Value::Null,
    }
}

fn timeout() -> Duration {
    // 8s gives CI runners ample headroom while still failing fast on a
    // wedged binary. Local dev typically completes each scenario in <500ms.
    Duration::from_secs(8)
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
fn ux_latency_open_then_completion() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP ux_latency_open_then_completion: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(e2e_config(timeout()))?;
    harness.open_file("latency.pl", SHORT_SOURCE)?;

    // Completion just after `my $other = $val` (cursor at end of partial var name).
    let items = harness
        .completion("latency.pl", 4, 16)
        .map_err(|e| anyhow::anyhow!("textDocument/completion errored under e2e config: {e}"))?;

    // E2E receipt: completion responded under e2e mode. Empty list is
    // acceptable: the receipt is "the request completed cleanly", not
    // "completion is high-quality" (that's the job of the dedicated
    // scenario_19 tests).
    let _ = items;
    harness.assert_no_crash();
    Ok(())
}

#[test]
fn ux_latency_open_then_hover() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP ux_latency_open_then_hover: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(e2e_config(timeout()))?;
    harness.open_file("latency.pl", SHORT_SOURCE)?;

    // Hover on `$value` (line 3 `my $value = 42;`, cursor inside the name).
    let _hover = harness
        .hover("latency.pl", 3, 5)
        .map_err(|e| anyhow::anyhow!("textDocument/hover errored under e2e config: {e}"))?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn ux_latency_edit_publishes_parse_error_diagnostic() -> Result<()> {
    if !binary_available() {
        eprintln!(
            "SKIP ux_latency_edit_publishes_parse_error_diagnostic: perl-lsp binary not found"
        );
        return Ok(());
    }

    let harness = UxHarness::new(e2e_config(timeout()))?;
    harness.open_file("broken.pl", PARSE_ERROR_SOURCE)?;

    // Under syntax-only + zero debounce, a parse error must arrive promptly.
    let diags = harness.wait_for_diagnostics("broken.pl", Duration::from_secs(5));
    assert!(
        !diags.is_empty(),
        "syntax-only e2e mode must surface parse errors; got empty diagnostics list"
    );

    // The diagnostic must be parser-sourced; syntax-only mode strips
    // critic / dead-code / module-resolution noise.
    let saw_parser =
        diags.iter().any(|d| d.get("source").and_then(|v| v.as_str()) == Some("perl-parser"));
    assert!(
        saw_parser,
        "expected at least one perl-parser diagnostic under syntax-only mode; got {diags:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn ux_latency_edit_clears_diagnostics_when_parse_recovers() -> Result<()> {
    if !binary_available() {
        eprintln!(
            "SKIP ux_latency_edit_clears_diagnostics_when_parse_recovers: perl-lsp binary not found"
        );
        return Ok(());
    }

    let harness = UxHarness::new(e2e_config(timeout()))?;
    harness.open_file("recovers.pl", PARSE_ERROR_SOURCE)?;

    let bad = harness.wait_for_diagnostics("recovers.pl", Duration::from_secs(5));
    assert!(!bad.is_empty(), "broken parse must report at least one diagnostic; got {bad:?}");

    // Apply the fix and expect the latest publish for this URI to be empty.
    harness.change_file_full("recovers.pl", CLEAN_SOURCE)?;
    let cleared = harness.wait_for_no_diagnostics("recovers.pl", Duration::from_secs(5));
    assert!(
        cleared,
        "syntax-only mode must publish an empty diagnostic list after the parse recovers"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn ux_latency_rapid_typing_latest_request_returns() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP ux_latency_rapid_typing_latest_request_returns: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(e2e_config(timeout()))?;
    harness.open_file("typing.pl", SHORT_SOURCE)?;

    // Simulate a short edit burst: each version replaces the file with a
    // longer variable name, growing one character at a time. This is the
    // typing-storm shape; every edit bumps the document generation, which
    // PR 4's generation-aware cancellation hooks onto.
    let burst = ["$va", "$val", "$valu", "$value"];
    for (i, partial) in burst.iter().enumerate() {
        let updated =
            format!("use strict;\nuse warnings;\n\nmy $value = 42;\nmy $other = {partial}\n");
        harness.change_file_full("typing.pl", &updated)?;
        // Throttle each edit just enough to give the scheduler real bursts
        // to deduplicate; on a wedged server this loop would time out.
        let _ = i;
        std::thread::sleep(Duration::from_millis(5));
    }

    // After the burst, the *latest* completion at the final cursor must
    // still return successfully. With PR 4, older queued completions are
    // cancelled; without PR 4, they may all run but the final answer is
    // what matters for the latency receipt.
    let _items: Vec<Value> = harness.completion("typing.pl", 4, 17)?;
    harness.assert_no_crash();
    Ok(())
}

#[test]
fn ux_latency_inline_completion_dynamic_path_returns_deterministic_items() -> Result<()> {
    if !binary_available() {
        eprintln!(
            "SKIP ux_latency_inline_completion_dynamic_path_returns_deterministic_items: perl-lsp binary not found"
        );
        return Ok(());
    }

    let harness = UxHarness::new(e2e_config(timeout()))?;
    let inline_registered =
        wait_for_registration(&harness, "textDocument/inlineCompletion", Duration::from_secs(2));
    assert!(
        inline_registered,
        "lean LSP4IJ-shaped client must receive dynamic inline-completion registration"
    );
    assert!(
        !registration_seen(&harness.client.peek_events(), "workspace/didChangeWatchedFiles"),
        "lean profile must not register file watchers even when the client supports dynamic watchers"
    );

    harness.open_file("inline.pl", "use ")?;
    let items = harness.inline_completion("inline.pl", 0, 4)?;
    assert!(!items.is_empty(), "inline completion must return deterministic items for `use `");
    assert!(
        items.iter().any(|item| item.get("insertText").and_then(Value::as_str) == Some("strict;")),
        "inline completion must include deterministic strict; suggestion, got {items:?}"
    );

    harness.assert_no_crash();
    Ok(())
}
