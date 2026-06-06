//! Scenario 18 — diagnostics refresh after textDocument/didChange.
//!
//! Verifies that the UX harness can drive a real edit cycle and observe
//! follow-up `textDocument/publishDiagnostics` updates for the edited file.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    LspEvent, ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario,
};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_18_diagnostics_after_edit.rs";

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
    run_ux_scenario(
        "diagnostics_after_edit_refresh",
        SCENARIO_FILE,
        "scenario_18_diagnostics_republish_after_full_document_edit",
        UxCiTier::Pr,
        Some(UxComponent::Diagnostics),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
                    .with_file("edit_diag.pl", BROKEN_SOURCE),
            )?;

            harness.open_file("edit_diag.pl", BROKEN_SOURCE)?;

            recorder.mark_request_start("initial_diagnostics");
            let initial = harness.wait_for_diagnostics("edit_diag.pl", Duration::from_secs(5));
            if !initial.is_empty() {
                recorder.mark_first_useful_result("initial_diagnostics");
            }
            recorder.check(
                "broken source published at least one initial diagnostic",
                !initial.is_empty(),
            )?;
            harness.collect_notifications();

            recorder.mark_request_start("diagnostics_after_full_document_edit");
            let updated = harness.apply_edit_and_collect_diagnostics(
                "edit_diag.pl",
                FIXED_SOURCE,
                Duration::from_secs(5),
            )?;
            recorder.mark_first_useful_result("diagnostics_after_full_document_edit");
            recorder.check(
                "updated diagnostics payload entries include range and message",
                updated
                    .iter()
                    .all(|diag| diag.get("range").is_some() && diag.get("message").is_some()),
            )?;

            recorder.mark_request_start("publish_diagnostics_after_edit");
            let uri = harness.workspace.uri("edit_diag.pl");
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut diagnostics_event_count = 0_usize;
            while Instant::now() < deadline {
                diagnostics_event_count = harness
                    .peek_notifications()
                    .iter()
                    .filter(|event| {
                        matches!(event, LspEvent::Diagnostics { uri: event_uri, .. } if event_uri == &uri)
                    })
                    .count();
                if diagnostics_event_count >= 1 {
                    recorder.mark_first_useful_result("publish_diagnostics_after_edit");
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }

            recorder.check(
                "publishDiagnostics republished after didChange edit",
                diagnostics_event_count >= 1,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
