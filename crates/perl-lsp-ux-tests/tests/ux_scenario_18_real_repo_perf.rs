//! Scenario 18 — Real-repo performance validation.
//!
//! Builds a representative Catalyst-like corpus from `test_corpus/real_world`
//! fixtures and validates time-to-first-diagnostics for a 5000+ line file.
#![cfg(feature = "integration-test")]

use anyhow::{Context, Result, bail};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    LspEvent, ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_18_real_repo_perf.rs";
const FIRST_DIAGNOSTICS_BUDGET: Duration = Duration::from_secs(5);
const MIN_LINES: usize = 5_000;
const REAL_WORLD_FIXTURES: &[&str] = &[
    "test_corpus/real_world/async_event_patterns.pl",
    "test_corpus/real_world/cli_text_processing.pl",
    "test_corpus/real_world/database_integration_patterns.pl",
    "test_corpus/real_world/enterprise_cpan_patterns.pl",
    "test_corpus/real_world/modern_oo_frameworks.pl",
    "test_corpus/real_world/testing_framework_patterns.pl",
    "test_corpus/real_world/web_framework_patterns.pl",
    "test_corpus/real_world/medium_module.pl",
];

fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("failed to resolve workspace root from CARGO_MANIFEST_DIR")
}

fn load_real_repo_fixture_source() -> Result<String> {
    let root = repo_root()?;
    let mut merged = String::new();

    merged.push_str("package MyApp::CatalystLike;\n");
    merged.push_str("use strict;\nuse warnings;\n\n");

    for fixture in REAL_WORLD_FIXTURES {
        let path = root.join(fixture);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read fixture source {}", path.display()))?;
        merged.push_str(&format!("# BEGIN FIXTURE: {fixture}\n"));
        merged.push_str(&content);
        if !content.ends_with('\n') {
            merged.push('\n');
        }
        merged.push_str(&format!("# END FIXTURE: {fixture}\n\n"));
    }

    merged.push_str("1;\n");

    let line_count = merged.lines().count();
    if line_count < MIN_LINES {
        bail!("merged real-world fixture source too small: {line_count} lines (< {MIN_LINES})");
    }

    Ok(merged)
}

#[test]
fn scenario_18_first_diagnostics_under_five_seconds_on_real_repo_fixture() {
    run_ux_scenario(
        "real_repo_first_diagnostics_budget",
        SCENARIO_FILE,
        "scenario_18_first_diagnostics_under_five_seconds_on_real_repo_fixture",
        UxCiTier::Nightly,
        Some(UxComponent::Diagnostics),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let source = load_real_repo_fixture_source()?;
            let config = ScenarioConfig { timeout: Duration::from_secs(30), ..Default::default() };
            let harness = UxHarness::new(config).context("failed to create UX harness")?;

            let relative_path = "lib/MyApp/CatalystLike.pm";
            let expected_uri = harness.workspace.uri(relative_path);

            recorder.mark_request_start("first_publish_diagnostics");
            let start = Instant::now();
            harness
                .open_file(relative_path, &source)
                .context("didOpen failed for real-repo performance fixture")?;

            let deadline = start + FIRST_DIAGNOSTICS_BUDGET;
            loop {
                let events = harness.peek_notifications();
                for event in &events {
                    if let LspEvent::Diagnostics { uri, .. } = event
                        && uri == &expected_uri
                    {
                        let latency = start.elapsed();
                        let within_budget = latency <= FIRST_DIAGNOSTICS_BUDGET;
                        if within_budget {
                            recorder.mark_first_useful_result("first_publish_diagnostics");
                        }
                        recorder.check(
                            "first publishDiagnostics arrives within 5 seconds",
                            within_budget,
                        )?;
                        harness.assert_no_crash();
                        return Ok(());
                    }
                }

                if Instant::now() >= deadline {
                    break;
                }

                std::thread::sleep(Duration::from_millis(25));
            }

            recorder.check("first publishDiagnostics arrives within 5 seconds", false)?;
            Ok(())
        },
    );
}
