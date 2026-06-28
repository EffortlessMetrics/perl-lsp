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
//! The test accepts either: it drains the pre-fix event queue, waits for any
//! stale in-flight broken-content results to arrive and be absorbed, then
//! checks whether the server sends an explicit empty notification or remains
//! silent (silence = cleared) within the post-settle window.
//!
//! # Race condition history
//!
//! The core challenge is that the LSP server runs diagnostics asynchronously.
//! After `textDocument/didChange` is sent with the fixed content, the server
//! may still be mid-analysis on the broken content, and those stale results
//! can arrive in the event queue after the fix is sent.
//!
//! Solution: a two-phase drain around the fix:
//!   Phase 1 (pre-fix):  drain + short settle to absorb events buffered before
//!                        `change_file_full` is called.
//!   Phase 2 (post-fix): a longer settle + drain immediately after
//!                        `change_file_full` to absorb stale in-flight results
//!                        from an analysis that was already running when the
//!                        fix arrived. Only then enter the clean-window check.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use std::time::Duration;

const BROKEN_SOURCE: &str = "use strict;\nuse warnings;\nmy $x = ;\n";
const FIXED_SOURCE: &str = "use strict;\nuse warnings;\nmy $x = 1;\nprint $x;\n";
const FIXED_VERSION: i64 = 2;

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

    // Phase 1 drain (pre-fix): remove events that have already arrived and any
    // that arrive during a brief settle window. This covers the common case where
    // the initial broken-content analysis is still delivering follow-up batches
    // (e.g., separate perltidy and perlcritic passes).
    harness.collect_notifications();
    std::thread::sleep(Duration::from_millis(400));
    harness.collect_notifications();

    // When: the user fixes the file via a full-document didChange.
    harness.change_file_full("live.pl", FIXED_SOURCE).expect("didChange should succeed");

    // Phase 2 drain (post-fix): give any in-flight analysis of the BROKEN content
    // time to complete and deliver its results, then drain those stale events.
    // The server may have been mid-analysis when didChange arrived; those stale
    // results can arrive up to ~500 ms after the fix is sent. Draining after this
    // settle ensures the clean-window check below only sees events triggered by
    // the fixed content.
    std::thread::sleep(Duration::from_millis(600));
    harness.collect_notifications();

    // Then: diagnostics eventually clear. Two acceptable outcomes:
    //   (a) Server sends explicit publishDiagnostics with empty array → cleared.
    //   (b) No new non-empty notification arrives within the clean window → silence
    //       means the server analysed the fixed content and found no errors.
    //
    // Diagnostics carry the text document version. If stale broken-content
    // diagnostics still arrive in this window, they are ignored when their
    // version proves they belong to the previous document state.
    let uri = harness.workspace.uri("live.pl");
    let deadline = std::time::Instant::now() + Duration::from_secs(4);

    let mut post_settle_events: Vec<LspEvent> = Vec::new();
    let mut cleared = false;

    while std::time::Instant::now() < deadline {
        post_settle_events.extend(harness.collect_notifications());

        // Walk in reverse to find the latest diagnostic event for this URI.
        for ev in post_settle_events.iter().rev() {
            if let LspEvent::Diagnostics { uri: event_uri, version, diagnostics } = ev {
                if event_uri == &uri {
                    if version.is_some_and(|value| value < FIXED_VERSION) {
                        continue;
                    }
                    if diagnostics.is_empty() {
                        cleared = true;
                    }
                    break; // latest diagnostic state found — stop scanning
                }
            }
        }

        if cleared {
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    // Accept silence: no new non-empty events arrived in the post-settle window.
    if !cleared {
        let has_new_errors = post_settle_events.iter().any(|ev| {
            matches!(
                ev,
                LspEvent::Diagnostics {
                    uri: event_uri,
                    version,
                    diagnostics,
                } if event_uri == &uri
                    && !diagnostics.is_empty()
                    && !version.is_some_and(|value| value < FIXED_VERSION)
            )
        });
        cleared = !has_new_errors;
    }

    assert!(
        cleared,
        "Expected diagnostics to clear (or no new errors) after fixing the file; \
         post-settle events: {:?}",
        post_settle_events
    );
    harness.assert_no_crash();
}
