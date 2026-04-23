// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — live diagnostics update cycle (open → edit → recover).
//!
//! This scenario hardens the UX harness around one of the highest-friction
//! first-session loops: the user types invalid Perl, sees diagnostics, fixes
//! the file, and expects diagnostics to clear.
//!
//! Acceptance criteria:
//! - Given a syntactically broken document, `didOpen` eventually yields at
//!   least one diagnostic.
//! - When the same document is fixed through `didChange`, the server eventually
//!   publishes an empty diagnostics set for that file.
//! - Then the server remains healthy (no crash signatures in notifications).

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

const BROKEN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub broken {\n\
    my $x = 1\n\
\n\
";

const FIXED_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub broken {\n\
    my $x = 1;\n\
}\n\
\n\
1;\n\
";

#[test]
fn scenario_19_given_broken_file_when_fixed_then_diagnostics_clear() {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return;
    }

    // Given: a fresh harness and a file with a parse-visible syntax defect.
    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("live_diagnostics.pl", BROKEN_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness
        .open_file("live_diagnostics.pl", BROKEN_SOURCE)
        .expect("didOpen should succeed for malformed documents");

    let initial_diagnostics = harness.wait_for_diagnostics_matching(
        "live_diagnostics.pl",
        Duration::from_secs(8),
        |diags| !diags.is_empty(),
    );

    assert!(
        initial_diagnostics.is_some(),
        "Expected non-empty diagnostics for malformed source after didOpen"
    );

    // When: the user fixes the same file in-editor (didChange full sync).
    harness.change_file("live_diagnostics.pl", FIXED_SOURCE).expect("didChange should succeed");

    // Then: diagnostics should eventually clear (publishDiagnostics with empty array).
    let cleared = harness.wait_for_diagnostics_matching(
        "live_diagnostics.pl",
        Duration::from_secs(8),
        |diags| diags.is_empty(),
    );

    assert!(
        cleared.is_some(),
        "Expected diagnostics to clear after fixing the document, but no empty diagnostics update arrived"
    );

    harness.assert_no_crash();
}
