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

const BROKEN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    my ($name) = @_;\n\
    print \"hello $name\\n\"\n\
}\n\
";

const FIXED_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    my ($name) = @_;\n\
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

    let _initial = harness.wait_for_diagnostics("edit_diag.pl", Duration::from_secs(5));

    harness
        .change_file_full("edit_diag.pl", FIXED_SOURCE)
        .expect("didChange full document should succeed");

    let updated = harness.wait_for_diagnostics("edit_diag.pl", Duration::from_secs(5));
    for diag in &updated {
        assert!(
            diag.get("range").is_some() && diag.get("message").is_some(),
            "Updated diagnostics payload must include range/message, got: {:?}",
            diag
        );
    }

    let uri = harness.workspace.uri("edit_diag.pl");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut diagnostics_event_count = 0_usize;
    while Instant::now() < deadline {
        diagnostics_event_count = harness
            .peek_notifications()
            .iter()
            .filter(|event| matches!(event, LspEvent::Diagnostics { uri: event_uri, .. } if event_uri == &uri))
            .count();
        if diagnostics_event_count >= 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    assert!(
        diagnostics_event_count >= 2,
        "Expected diagnostics to publish on open and republish after edit; observed {} events",
        diagnostics_event_count
    );
    harness.assert_no_crash();
}
