// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 01 — Clean install, simple file.
//!
//! Simulates the very first thing a user does after installing perl-lsp:
//! open a trivial `.pl` file and verify the server responds to hover.
//!
//! Acceptance criteria:
//! - Server starts without crashing.
//! - `textDocument/didOpen` is accepted (no error).
//! - `textDocument/hover` on a variable returns something, or null in degraded mode.
//! - No crash signatures in the event log.
//! - Completion request does not crash.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};

#[test]
fn scenario_01_server_starts_and_accepts_open() {
    run_ux_scenario(
        "simple_file_smoke",
        "ux_scenario_01_simple_file.rs",
        "scenario_01_server_starts_and_accepts_open",
        UxCiTier::Pr,
        Some(UxComponent::Infra),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let source = "#!/usr/bin/env perl\nuse strict;\n\nprint \"Hello, world!\\n\";\n";

            let harness = UxHarness::new(ScenarioConfig::default())?;

            harness.open_file("hello.pl", source)?;

            recorder.check("didOpen accepted without error", true)?;

            harness.assert_no_crash();
            recorder.check("no crash signatures in event log", true)?;

            Ok(())
        },
    );
}

#[test]
fn scenario_01_hover_on_simple_variable() {
    run_ux_scenario(
        "simple_file_smoke",
        "ux_scenario_01_simple_file.rs",
        "scenario_01_hover_on_simple_variable",
        UxCiTier::Pr,
        Some(UxComponent::Hover),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let source = "use strict;\nuse warnings;\n\nmy $x = 42;\nmy $y = $x + 1;\n";

            let harness = UxHarness::new(ScenarioConfig::default().with_file("test.pl", source))?;

            harness.open_file("test.pl", source)?;

            // Hover on `$x` (line 3, character 3).
            recorder.mark_request_start("hover");
            let hover_result = harness.hover("test.pl", 3, 3);

            match hover_result {
                Ok(Some(result)) => {
                    recorder.mark_first_useful_result("hover");
                    recorder.check(
                        "hover result is an object or string",
                        result.is_object() || result.is_string(),
                    )?;
                }
                Ok(None) => {
                    // Degraded mode — hover returned null. Still a useful
                    // (expected-clean) result for timing purposes.
                    recorder.mark_first_useful_result("hover");
                    recorder.check("hover returned null (degraded mode acceptable)", true)?;
                }
                Err(e) => {
                    let _ = recorder.check("hover should not return a JSON-RPC error", false);
                    anyhow::bail!("Hover returned a JSON-RPC error — this is a UX regression: {e}");
                }
            }

            harness.assert_no_crash();
            recorder.check("no crash signatures in event log", true)?;

            Ok(())
        },
    );
}

#[test]
fn scenario_01_completion_on_keyword_does_not_crash() {
    run_ux_scenario(
        "simple_file_smoke",
        "ux_scenario_01_simple_file.rs",
        "scenario_01_completion_on_keyword_does_not_crash",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let source = "use str\n";
            let harness = UxHarness::new(ScenarioConfig::default())?;

            harness.open_file("complete.pl", source)?;

            recorder.mark_request_start("completion");
            let result = harness.completion("complete.pl", 0, 7);
            recorder.check("completion request did not crash", result.is_ok())?;
            recorder.mark_first_useful_result("completion");

            Ok(())
        },
    );
}
