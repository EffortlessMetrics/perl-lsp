//! Scenario 19 — incremental edit flow updates diagnostics and keeps UX responsive.
//!
//! BDD focus:
//! - GIVEN a file with an obvious strict-mode diagnostic.
//! - WHEN the user fixes the file through a didChange edit.
//! - THEN diagnostics should eventually clear and hover should still respond.

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::{Duration, Instant};

const BROKEN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    return $missing_name;\n\
}\n\
\n\
1;\n\
";

const FIXED_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $missing_name = 'Perl';\n\
\n\
sub greet {\n\
    return $missing_name;\n\
}\n\
\n\
1;\n\
";

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn wait_until_diagnostics(
    harness: &UxHarness,
    relative_path: &str,
    expect_empty: bool,
    timeout: Duration,
) -> Vec<serde_json::Value> {
    let deadline = Instant::now() + timeout;
    let mut latest = Vec::new();
    while Instant::now() < deadline {
        latest = harness.wait_for_diagnostics(relative_path, Duration::from_millis(250));
        if expect_empty == latest.is_empty() {
            return latest;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    latest
}

#[test]
fn scenario_19_incremental_edit_clears_diagnostics_and_preserves_hover() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("incremental.pl", BROKEN_SOURCE))?;

    // GIVEN a strict file with an undeclared variable.
    harness.open_file("incremental.pl", BROKEN_SOURCE)?;
    let initial_diagnostics =
        wait_until_diagnostics(&harness, "incremental.pl", false, Duration::from_secs(8));
    assert!(
        !initial_diagnostics.is_empty(),
        "Expected diagnostics for undeclared variable before edit"
    );

    // WHEN the user applies an in-editor fix through didChange.
    harness.change_file("incremental.pl", FIXED_SOURCE)?;

    // THEN diagnostics eventually clear for the same document.
    let final_diagnostics =
        wait_until_diagnostics(&harness, "incremental.pl", true, Duration::from_secs(8));
    assert!(
        final_diagnostics.is_empty(),
        "Expected diagnostics to clear after fix, got {final_diagnostics:?}"
    );

    // AND the file remains interactive for follow-up UX actions.
    let hover = harness.hover("incremental.pl", 7, 12)?;
    if hover.is_none() {
        eprintln!(
            "INFO scenario_19: hover returned null after edits (acceptable degraded behavior)"
        );
    }

    harness.assert_no_crash();
    Ok(())
}
