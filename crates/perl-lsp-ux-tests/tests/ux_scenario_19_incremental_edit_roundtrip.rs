//! Scenario 19 — incremental edit roundtrip UX.
//!
//! Ensures the UX harness can drive real `didChange` flows and observe
//! post-change diagnostics transitions, which are core to day-to-day editing.

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

const BROKEN_SOURCE: &str =
    "use strict;\nuse warnings;\n\nmy $value = $missing + 1;\nprint $value;\n";
const FIXED_SOURCE: &str =
    "use strict;\nuse warnings;\n\nmy $missing = 41;\nmy $value = $missing + 1;\nprint $value;\n";

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn diagnostic_count(diags: &[serde_json::Value]) -> usize {
    diags.len()
}

#[test]
fn scenario_19_did_change_refreshes_diagnostics() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19_did_change_refreshes_diagnostics: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() })?;

    harness.open_file("edit_cycle.pl", BROKEN_SOURCE)?;

    let first_diags = harness.wait_for_diagnostics("edit_cycle.pl", Duration::from_secs(8));
    let first_count = diagnostic_count(&first_diags);
    assert!(first_count > 0, "expected diagnostics before fix, got none: {:?}", first_diags);

    let baseline_events = harness.diagnostics_event_count("edit_cycle.pl");
    harness.change_file_full("edit_cycle.pl", FIXED_SOURCE)?;

    let second_diags = harness.wait_for_diagnostics_after(
        "edit_cycle.pl",
        baseline_events,
        Duration::from_secs(8),
    );
    let second_count = diagnostic_count(&second_diags);

    assert!(
        second_count <= first_count,
        "expected diagnostics to improve or stay stable after fix; before={}, after={}, payload={:?}",
        first_count,
        second_count,
        second_diags
    );

    harness.assert_no_crash();
    Ok(())
}
