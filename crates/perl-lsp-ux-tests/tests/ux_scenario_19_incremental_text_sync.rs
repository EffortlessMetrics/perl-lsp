//! Scenario 19 — incremental text-sync UX regression coverage.
//!
//! BDD workflow:
//! - Given an open Perl document with a parse error,
//! - When the user fixes the document and the editor emits didChange,
//! - Then diagnostics should recover and hover should return useful contents.

use anyhow::{Context, Result, ensure};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::{Duration, Instant};

const BROKEN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $value = ;\n\
print $value;\n\
";

const FIXED_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $value = 42;\n\
print $value;\n\
";

fn has_parse_like_diagnostic(diagnostics: &[serde_json::Value]) -> bool {
    diagnostics.iter().any(|diag| {
        let message = diag
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        message.contains("syntax")
            || message.contains("parse")
            || message.contains("unexpected")
            || message.contains("expected")
    })
}

fn hover_contents_text(contents: &Value) -> String {
    if let Some(value) = contents.get("value").and_then(Value::as_str) {
        return value.to_string();
    }

    if let Some(text) = contents.as_str() {
        return text.to_string();
    }

    if let Some(items) = contents.as_array() {
        return items.iter().map(hover_contents_text).collect::<Vec<_>>().join("\n");
    }

    String::new()
}

fn wait_for_any_diagnostics_event(
    harness: &UxHarness,
    timeout: Duration,
) -> Option<Vec<serde_json::Value>> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for event in harness.peek_notifications() {
            if let LspEvent::Diagnostics { diagnostics, .. } = event {
                return Some(diagnostics);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

#[test]
fn scenario_19_didchange_recovers_after_parse_error_fix() -> Result<()> {
    if !binary_available() {
        tracing::info!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("sync.pl", BROKEN_SOURCE),
    )
    .context("Failed to create UX harness")?;

    // Given: user opens a file that initially has parse issues.
    harness.open_file("sync.pl", BROKEN_SOURCE).context("didOpen should succeed")?;
    let initial_diagnostics = harness.wait_for_diagnostics("sync.pl", Duration::from_secs(5));

    // The GIVEN clause is load-bearing: if the server never reports a
    // parse-like problem for `my $value = ;` the rest of the test is
    // meaningless (we cannot prove "recovery" without a prior broken state).
    // We require at least one diagnostic; we keep the parse-keyword check as
    // a soft preference so wording changes in the server don't break the test,
    // but we refuse to silently continue when the server emitted nothing.
    ensure!(
        !initial_diagnostics.is_empty(),
        "GIVEN preconditions failed: expected at least one diagnostic for \
         `my $value = ;` before the fix, got empty. Recovery assertion would \
         otherwise pass vacuously."
    );

    // Clear previously buffered notifications so post-change assertions inspect fresh server output.
    let _ = harness.collect_notifications();

    // When: user fixes the file and the editor sends didChange full-text sync.
    harness.change_file_full("sync.pl", FIXED_SOURCE).context("didChange should succeed")?;

    // Then: diagnostics should settle without parse-like errors and server remains responsive.
    let post_change_diagnostics = harness.wait_for_diagnostics("sync.pl", Duration::from_secs(5));
    ensure!(
        !has_parse_like_diagnostic(&post_change_diagnostics),
        "expected parse-like diagnostics to clear after fixing file; got: {:?}",
        post_change_diagnostics
    );
    // And the diagnostic set must have strictly shrunk or changed — if we
    // see an identical list we didn't actually recover from anything.
    ensure!(
        post_change_diagnostics != initial_diagnostics,
        "diagnostics after fix are identical to before; didChange had no effect"
    );

    // Hover must return useful content after the editor fixes the file; a
    // clean-but-null result would leave the user without feedback after typing.
    let hover = harness
        .hover("sync.pl", 4, 8)
        .context("hover should not JSON-RPC error after didChange recovery")?;
    let Some(hover) = hover else {
        anyhow::bail!("hover returned null after didChange recovery");
    };
    let contents =
        hover.get("contents").context("hover after didChange recovery missing contents")?;
    let contents_text = hover_contents_text(contents);
    ensure!(
        !contents_text.trim().is_empty(),
        "hover after didChange recovery should return non-empty contents; got: {:?}",
        contents
    );

    let diagnostics_event = wait_for_any_diagnostics_event(&harness, Duration::from_secs(1));
    ensure!(
        diagnostics_event.is_some(),
        "expected at least one publishDiagnostics event during didChange recovery"
    );

    harness.assert_no_crash();
    Ok(())
}
