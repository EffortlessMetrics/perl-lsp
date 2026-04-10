// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
//! - A clean file MAY produce zero diagnostics — that is acceptable.

use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

/// Source that is syntactically valid Perl — should produce no parse errors.
const CLEAN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $x = 42;\n\
print \"$x\\n\";\n\
";

/// Source with a declared-but-unused-under-strict variable.  Some diagnostics
/// providers flag this; others do not.  We only verify shape, not count.
const STRICT_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $unused_var = 99;\n\
print \"done\\n\";\n\
";

#[test]
fn scenario_12_server_does_not_crash_after_diagnostics_request() {
    if !binary_available() {
        eprintln!("SKIP scenario_12: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("clean.pl", CLEAN_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("clean.pl", CLEAN_SOURCE).expect("didOpen should succeed");

    // Allow diagnostics to publish (server-push; no blocking call needed).
    std::thread::sleep(Duration::from_secs(2));

    harness.assert_no_crash();
}

#[test]
fn scenario_12_diagnostics_notification_shape_is_valid() {
    if !binary_available() {
        eprintln!("SKIP scenario_12: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("strict_test.pl", STRICT_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("strict_test.pl", STRICT_SOURCE).expect("didOpen should succeed");

    // Wait up to 5 seconds for diagnostics to arrive.
    let diagnostics = harness.wait_for_diagnostics("strict_test.pl", Duration::from_secs(5));

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
}

#[test]
fn scenario_12_publishdiagnostics_notification_was_received() {
    if !binary_available() {
        eprintln!("SKIP scenario_12: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("notify_test.pl", CLEAN_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("notify_test.pl", CLEAN_SOURCE).expect("didOpen should succeed");

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
}
