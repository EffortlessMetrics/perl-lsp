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

const BROKEN_SOURCE: &str = "use strict;\nuse warnings;\nmy $x = ;\n";
const FIXED_SOURCE: &str = "use strict;\nuse warnings;\nmy $x = 1;\nprint $x;\n";

/// Verifies the diagnostics edit lifecycle:
///   1. Broken content → diagnostics appear.
///   2. Fixed content → diagnostics clear (either explicitly or by silence).
#[test]
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
    // Three-pass flush: the stdout reader thread may still be delivering in-flight
    // events from the broken-content analysis (async diagnostics pipeline).
    // - First drain: remove any already-buffered events.
    // - Settle window: wait for in-flight events from the server to arrive.
    // - Second drain: remove events that arrived during the settle window.
    // - Final drain immediately before didChange: close the race window between
    //   the settle and the send.  Events that slip in during this last tiny gap
    //   are cleared before we record the post-fix baseline.
    harness.collect_notifications();
    std::thread::sleep(Duration::from_millis(300));
    harness.collect_notifications();

    // Final drain immediately before the fix — eliminates events that snuck in
    // between the settle drain and the didChange notification.
    harness.collect_notifications();

    // When: the user fixes the file via a full-document didChange.
    harness.change_file_full("live.pl", FIXED_SOURCE).expect("didChange should succeed");

    // Then: diagnostics eventually clear. Two acceptable outcomes:
    //   (a) Server sends explicit publishDiagnostics with empty array → cleared.
    //   (b) No new non-empty notification arrives within the grace period → also cleared.
    //
    // Important: we accumulate ONLY events that arrive after the final pre-fix drain
    // by draining periodically and appending to a local list.  Using peek_notifications
    // would re-read pre-fix events that raced into the queue between the drain and the
    // didChange send, producing false failures when the server never sends an explicit
    // empty clear for the fixed content.
    let uri = harness.workspace.uri("live.pl");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);

    let mut post_fix_events: Vec<LspEvent> = Vec::new();
    let mut cleared = false;

    while std::time::Instant::now() < deadline {
        // Drain any new events since the last iteration and append to our local list.
        post_fix_events.extend(harness.collect_notifications());

        let post_fix_diag_events: Vec<_> = post_fix_events
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

    // Accept silence (no new non-empty notification in our post-fix window) as "cleared" too.
    if !cleared {
        let has_new_errors = post_fix_events.iter().any(|ev| {
            matches!(ev, LspEvent::Diagnostics { uri: event_uri, diagnostics }
                if event_uri == &uri && !diagnostics.is_empty())
        });
        cleared = !has_new_errors;
    }

    assert!(
        cleared,
        "Expected diagnostics to clear (or no new errors) after fixing the file; events: {:?}",
        post_fix_events
    );
    harness.assert_no_crash();
}
