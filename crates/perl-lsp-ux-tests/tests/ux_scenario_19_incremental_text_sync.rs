//! Scenario 19 — incremental text-sync UX regression coverage.
//!
//! BDD workflow:
//! - Given an open Perl document with a parse error,
//! - When the user fixes the document and the editor emits didChange,
//! - Then diagnostics should recover and the server should keep serving requests.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    LspEvent, ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario,
};
use serde_json::Value;
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_19_incremental_text_sync.rs";

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

fn has_parse_like_diagnostic(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|diag| {
        let message = diag
            .get("message")
            .and_then(Value::as_str)
            .map_or_else(String::new, str::to_ascii_lowercase);
        message.contains("syntax")
            || message.contains("parse")
            || message.contains("unexpected")
            || message.contains("expected")
    })
}

fn wait_for_any_diagnostics_event(harness: &UxHarness, timeout: Duration) -> Option<Vec<Value>> {
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
fn scenario_19_didchange_recovers_after_parse_error_fix() {
    run_ux_scenario(
        "incremental_text_sync_recovery",
        SCENARIO_FILE,
        "scenario_19_didchange_recovers_after_parse_error_fix",
        UxCiTier::Pr,
        Some(UxComponent::Infra),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
                    .with_file("sync.pl", BROKEN_SOURCE),
            )?;

            recorder.mark_request_start("initial_parse_diagnostics");
            harness.open_file("sync.pl", BROKEN_SOURCE)?;
            let initial_diagnostics =
                harness.wait_for_diagnostics("sync.pl", Duration::from_secs(5));
            let has_initial_diagnostics = !initial_diagnostics.is_empty();
            if has_initial_diagnostics {
                recorder.mark_first_useful_result("initial_parse_diagnostics");
            }
            recorder.check(
                "broken content yields at least one diagnostic before fix",
                has_initial_diagnostics,
            )?;

            let _ = harness.collect_notifications();

            harness.change_file_full("sync.pl", FIXED_SOURCE)?;

            recorder.mark_request_start("post_change_diagnostics_recovery");
            let post_change_diagnostics =
                harness.wait_for_diagnostics("sync.pl", Duration::from_secs(5));
            let parse_like_cleared = !has_parse_like_diagnostic(&post_change_diagnostics);
            let diagnostics_changed = post_change_diagnostics != initial_diagnostics;
            if parse_like_cleared && diagnostics_changed {
                recorder.mark_first_useful_result("post_change_diagnostics_recovery");
            }
            recorder.check(
                "fixed content clears parse-like diagnostics after didChange",
                parse_like_cleared,
            )?;
            recorder.check(
                "diagnostics after fix differ from the broken-content diagnostics",
                diagnostics_changed,
            )?;

            recorder.mark_request_start("post_change_hover");
            let hover_result = harness.hover("sync.pl", 4, 2);
            if hover_result.is_ok() {
                recorder.mark_first_useful_result("post_change_hover");
            }
            recorder.check(
                "hover does not JSON-RPC error after didChange recovery",
                hover_result.is_ok(),
            )?;

            recorder.mark_request_start("post_change_publish_diagnostics_event");
            let diagnostics_event =
                wait_for_any_diagnostics_event(&harness, Duration::from_secs(1));
            if diagnostics_event.is_some() {
                recorder.mark_first_useful_result("post_change_publish_diagnostics_event");
            }
            recorder.check(
                "publishDiagnostics event is observed during didChange recovery",
                diagnostics_event.is_some(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
