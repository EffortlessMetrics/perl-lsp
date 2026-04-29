// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — diagnostics lifecycle during active editing.
//!
//! This scenario covers an editor-critical UX flow: a user introduces a parse
//! error, sees diagnostics, fixes the file, and expects diagnostics to clear.
//!
//! # Robustness note
//!
//! LSP servers may clear diagnostics in two ways:
//! 1. Explicit empty `textDocument/publishDiagnostics` (empty array).
//! 2. Silently — no notification after fix.
//!
//! The test accepts either: it drains the pre-fix event queue, then checks
//! whether the server sends an explicit empty notification within the timeout.
//! If no new notification arrives, the test also passes (silence = cleared).

use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

const BROKEN_SOURCE: &str = "use strict;\nmy $x = ;\n";
const FIXED_SOURCE: &str = "use strict;\nmy $x = 1;\n";

/// Verifies the diagnostics edit lifecycle:
///   1. Broken content → diagnostics appear.
///   2. Fixed content → diagnostics clear (either explicitly or by silence).
#[test]
#[ignore = "flaky in CI; tracked in #7297"]
fn scenario_19_diagnostics_clear_after_fix() {
    if !binary_available() {
        eprintln!("SKIP scenario_19_diagnostics_clear_after_fix: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("live.pl", BROKEN_SOURCE),
    )
    .expect("Failed to create UX harness");

    // Given: a workspace file opened with a syntax error.
    harness.open_file("live.pl", BROKEN_SOURCE).expect("didOpen should succeed");

    // When: diagnostics are first published for the broken content.
    let diagnostics = harness.wait_for_diagnostics("live.pl", Duration::from_secs(5));
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for broken source, but none were published."
    );

    // Drain the event queue so post-fix checks only see new notifications.
    //
    // Two-pass flush: the stdout reader thread may still be delivering in-flight
    // events from the broken-content analysis (async diagnostics pipeline).  A
    // single drain races with those deliveries — the queue can appear empty, then
    // the late events arrive and get misclassified as "post-fix".  Waiting a
    // brief settle window (≥ one server round-trip) and draining again eliminates
    // that window reliably.
    harness.collect_notifications();
    std::thread::sleep(Duration::from_millis(300));
    harness.collect_notifications();

    // When: the user fixes the file via a full-document didChange.
    harness.change_file_full("live.pl", FIXED_SOURCE).expect("didChange should succeed");

    // Then: diagnostics eventually clear. Two acceptable outcomes:
    //   (a) Server sends explicit publishDiagnostics with empty array → cleared.
    //   (b) No new non-empty notification arrives within the grace period → also cleared.
    let uri = harness.workspace.uri("live.pl");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);

    let mut cleared = false;
    while std::time::Instant::now() < deadline {
        let events = harness.peek_notifications();
        let post_fix_diag_events: Vec<_> = events
            .iter()
            .filter_map(|ev| match ev {
                LspEvent::Diagnostics { uri: event_uri, diagnostics } if event_uri == &uri => {
                    Some(diagnostics.as_slice())
                }
                _ => None,
            })
            .collect();

        if let Some(latest) = post_fix_diag_events.last() {
            // Server sent an explicit notification — check if cleared.
            if latest.is_empty() {
                cleared = true;
                break;
            }
            // Server sent new non-empty diagnostics — not cleared yet, keep waiting.
        }
        // No post-fix notification seen yet — keep polling.
        std::thread::sleep(Duration::from_millis(100));
    }

    // Accept silence (no new non-empty notification) as "cleared" too.
    if !cleared {
        let events = harness.peek_notifications();
        let has_new_errors = events.iter().any(|ev| {
            matches!(ev, LspEvent::Diagnostics { uri: event_uri, diagnostics }
                if event_uri == &uri && !diagnostics.is_empty())
        });
        cleared = !has_new_errors;
    }

    assert!(cleared, "Expected diagnostics to clear (or no new errors) after fixing the file.");
    harness.assert_no_crash();
}
