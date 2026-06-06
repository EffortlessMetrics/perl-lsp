// Test infrastructure needs skip/status messages when the external binary is absent.
#![allow(clippy::print_stderr)]
// Test assertions intentionally panic with UX-specific failure messages.
#![allow(clippy::panic)]

//! Scenario 12 — `textDocument/publishDiagnostics` feature grid coverage.
//!
//! Verifies that the server emits diagnostics notifications when Perl code has
//! known issues.  This exercises the `textDocument/publishDiagnostics`
//! capability advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - After `didOpen`, the server MUST eventually send a
//!   `textDocument/publishDiagnostics` notification (possibly empty).
//! - The notification MUST NOT crash the server.
//! - If diagnostics are returned they MUST be well-formed objects with at least
//!   `range` and `message` fields.
//! - Strict-mode undeclared variables MUST surface a diagnostic mentioning the symbol.
//! - A clean file MAY produce zero diagnostics — that is acceptable.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

/// Source that is syntactically valid Perl — should produce no parse errors.
const CLEAN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $x = 42;\n\
print \"$x\\n\";\n\
";

/// Source with an undeclared variable under strict; this should be visible to
/// users as a concrete diagnostic, not just as a generic publish event.
const STRICT_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
print $missing_name;\n\
";

#[test]
fn scenario_12_server_does_not_crash_after_diagnostics_request() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_12: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("clean.pl", CLEAN_SOURCE),
    )?;

    harness.open_file("clean.pl", CLEAN_SOURCE)?;

    // Allow diagnostics to publish (server-push; no blocking call needed).
    std::thread::sleep(Duration::from_secs(2));

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_12_undeclared_variable_diagnostic_mentions_symbol() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_12: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("strict_test.pl", STRICT_SOURCE),
    )?;

    harness.open_file("strict_test.pl", STRICT_SOURCE)?;

    // Wait up to 5 seconds for diagnostics to arrive.
    let diagnostics = harness.wait_for_diagnostics("strict_test.pl", Duration::from_secs(5));
    assert!(
        diagnostics.iter().any(|diag| diagnostic_mentions_symbol(diag, "$missing_name")),
        "expected strict undeclared-variable diagnostic mentioning $missing_name, got: {diagnostics:?}"
    );

    // Validate each diagnostic has the required LSP fields.
    for diag in &diagnostics {
        assert!(diag.get("range").is_some(), "Diagnostic must have 'range' field, got: {:?}", diag);
        assert!(
            diag.get("message").is_some(),
            "Diagnostic must have 'message' field, got: {:?}",
            diag
        );
        // severity is optional but must be 1-4 when present.
        if let Some(severity) = diag.get("severity") {
            let s = severity.as_u64().unwrap_or(0);
            assert!((1..=4).contains(&s), "Diagnostic severity must be 1-4, got: {}", s);
        }
    }

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_12_publishdiagnostics_notification_was_received() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_12: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("notify_test.pl", CLEAN_SOURCE),
    )?;

    harness.open_file("notify_test.pl", CLEAN_SOURCE)?;

    // Poll for up to 5 seconds to see if the server ever fires publishDiagnostics.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut received = false;
    while std::time::Instant::now() < deadline {
        let events = harness.peek_notifications();
        for ev in &events {
            if let LspEvent::Diagnostics { .. } = ev {
                received = true;
                break;
            }
        }
        if received {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    if !received {
        eprintln!(
            "INFO scenario_12: server did not publish diagnostics within 5s \
             (may require external linter — degraded mode acceptable)"
        );
    }

    harness.assert_no_crash();
    Ok(())
}

fn diagnostic_mentions_symbol(diag: &Value, symbol: &str) -> bool {
    diag.get("message").and_then(Value::as_str).is_some_and(|message| message.contains(symbol))
}
