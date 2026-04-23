//! Scenario 19 — diagnostics lifecycle during active editing.
//!
//! This scenario covers an editor-critical UX flow: a user introduces a parse
//! error, sees diagnostics, fixes the file, and expects diagnostics to clear.

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

const BROKEN_SOURCE: &str = "use strict;\nmy $x = ;\n";
const FIXED_SOURCE: &str = "use strict;\nmy $x = 1;\n";

#[test]
fn scenario_19_diagnostics_clear_after_fix() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19_diagnostics_clear_after_fix: perl-lsp binary not found");
        return Ok(());
    }

    // Given: a workspace file opened with a syntax error.
    let harness = UxHarness::new(ScenarioConfig::default().with_file("live.pl", BROKEN_SOURCE))?;
    harness.open_file("live.pl", BROKEN_SOURCE)?;

    // When: diagnostics are first published for the broken content.
    let diagnostics = harness.wait_for_diagnostics("live.pl", Duration::from_secs(5));
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for broken source, but none were published."
    );

    // When: the user fixes the file via a full-document didChange.
    harness.change_file_full("live.pl", FIXED_SOURCE)?;

    // Then: diagnostics eventually clear for the same document.
    let cleared = harness.wait_for_no_diagnostics("live.pl", Duration::from_secs(5));
    assert!(cleared, "Expected diagnostics to clear after fixing the file.");
    harness.assert_no_crash();
    Ok(())
}
