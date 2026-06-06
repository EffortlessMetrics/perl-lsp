//! Scenario 06 — Large file.
//!
//! Opens a 10 000-line Perl file and verifies that the server handles it without
//! hanging or OOM-crashing.
//!
//! The heavy tests (10k lines) are gated behind `integration-test` feature.
//! The gate allows the scenario to appear in the default test run (with a
//! reduced 1k-line version) so CI always exercises the code path.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_06_large_file.rs";

fn generate_source(line_count: usize) -> String {
    let mut buf = String::with_capacity(line_count * 40);
    buf.push_str("use strict;\nuse warnings;\n\n");
    for i in 0..line_count {
        buf.push_str(&format!("sub func_{i} {{ my $x_{i} = {i}; return $x_{i}; }}\n"));
    }
    buf
}

fn run_large_file_workflow(test_name: &str, file_name: &str, line_count: usize, timeout: Duration) {
    run_ux_scenario(
        "large_file_open",
        SCENARIO_FILE,
        test_name,
        UxCiTier::Nightly,
        Some(UxComponent::Infra),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let source = generate_source(line_count);
            let harness = UxHarness::new(ScenarioConfig { timeout, ..Default::default() })?;

            recorder.mark_request_start("open_then_hover");
            harness.open_file(file_name, &source)?;

            let hover = harness.hover(file_name, 5, 5);
            let hover_ok = hover.is_ok();
            if hover_ok {
                recorder.mark_first_useful_result("open_then_hover");
            }
            recorder.check("hover after large-file open does not error", hover_ok)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_06_medium_file_open_and_hover() {
    // Always runs — 1k lines is fast enough for PR gate.
    run_large_file_workflow(
        "scenario_06_medium_file_open_and_hover",
        "medium.pl",
        1_000,
        Duration::from_secs(20),
    );
}

#[cfg(feature = "integration-test")]
#[test]
fn scenario_06_large_file_open_does_not_hang() {
    run_large_file_workflow(
        "scenario_06_large_file_open_does_not_hang",
        "large.pl",
        10_000,
        Duration::from_secs(30),
    );
}
