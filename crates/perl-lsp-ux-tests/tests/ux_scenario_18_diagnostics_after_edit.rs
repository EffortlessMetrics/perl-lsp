// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 18 — diagnostics refresh after textDocument/didChange.
//!
//! Verifies that the UX harness can drive a real edit cycle and observe
//! follow-up `textDocument/publishDiagnostics` updates for the edited file.

use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use std::time::{Duration, Instant};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

// NOTE: BROKEN_SOURCE contains a genuine Perl syntax error — the incomplete
// expression `(1 +` triggers a parse failure under `use strict`.  A missing
// semicolon at the end of a `print` statement is *not* a syntax error in Perl
// (the next `}` terminates the statement), so we use an unterminated expression
// instead to guarantee the server publishes at least one diagnostic.
const BROKEN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    my ($name) = @_;\n\
    my $broken = (1 + ;\n\
    print \"hello $name\\n\";\n\
}\n\
";

const FIXED_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    my ($name) = @_;\n\
    my $ok = (1 + 2);\n\
    print \"hello $name\\n\";\n\
}\n\
";

#[test]
fn scenario_18_diagnostics_republish_after_full_document_edit() {
    if !binary_available() {
        eprintln!("SKIP scenario_18: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("edit_diag.pl", BROKEN_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("edit_diag.pl", BROKEN_SOURCE).expect("didOpen should succeed");

    // Wait for and then drain the initial diagnostics so the post-edit wait
    // sees only new events, not the pre-edit ones still in the peek queue.
    let initial = harness.wait_for_diagnostics("edit_diag.pl", Duration::from_secs(5));
    assert!(
        !initial.is_empty(),
        "Expected at least one diagnostic for BROKEN_SOURCE (unterminated expression); \
         got none — check the fixture content"
    );
    // Drain so the next wait_for_diagnostics sees only the post-edit notification.
    harness.collect_notifications();

    let updated = harness
        .apply_edit_and_collect_diagnostics("edit_diag.pl", FIXED_SOURCE, Duration::from_secs(5))
        .expect("didChange full document should succeed");
    for diag in &updated {
        assert!(
            diag.get("range").is_some() && diag.get("message").is_some(),
            "Updated diagnostics payload must include range/message, got: {:?}",
            diag
        );
    }

    // After draining the open-event and sending didChange, wait for the server
    // to republish diagnostics.  We expect at least 1 new diagnostics event.
    let uri = harness.workspace.uri("edit_diag.pl");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut diagnostics_event_count = 0_usize;
    while Instant::now() < deadline {
        diagnostics_event_count = harness
            .peek_notifications()
            .iter()
            .filter(|event| matches!(event, LspEvent::Diagnostics { uri: event_uri, .. } if event_uri == &uri))
            .count();
        if diagnostics_event_count >= 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    assert!(
        diagnostics_event_count >= 1,
        "Expected diagnostics to republish after didChange edit; observed {} post-edit events",
        diagnostics_event_count
    );
    harness.assert_no_crash();
}
