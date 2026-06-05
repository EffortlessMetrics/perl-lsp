//! Scenario 24 — Live-edit UX feedback loop for diagnostics + definition.
//!
//! BDD contract:
//! - Given a file with an undefined variable, when it is opened, then diagnostics
//!   should surface a strict warning/error for that variable.
//! - Given the declaration was added, when go-to-definition runs on the use-site,
//!   then it should stay responsive and return either locations or an empty
//!   result (degraded mode).

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_24_live_edit_feedback.rs";

const UNDECLARED_SOURCE: &str = r#"use strict;
use warnings;

print $name;
"#;

const DECLARED_SOURCE: &str = r#"use strict;
use warnings;

my $name = 'world';
print $name;
"#;

fn create_live_edit_harness() -> Result<UxHarness> {
    UxHarness::new(ScenarioConfig::default().with_file("live_edit.pl", UNDECLARED_SOURCE))
}

fn has_global_symbol_diagnostic(diags: &[Value], symbol: &str) -> bool {
    diags.iter().any(|diag| {
        let message = diag.get("message").and_then(Value::as_str).unwrap_or_default();
        let code = diag.get("code").and_then(Value::as_str).unwrap_or_default();
        message.contains(symbol) || (code.contains("Global symbol") && message.contains(symbol))
    })
}

#[test]
fn given_undeclared_variable_when_opened_then_strict_diagnostic_is_published() {
    run_ux_scenario(
        "live_edit_feedback_loop",
        SCENARIO_FILE,
        "given_undeclared_variable_when_opened_then_strict_diagnostic_is_published",
        UxCiTier::Pr,
        Some(UxComponent::Diagnostics),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_live_edit_harness()?;
            recorder.mark_request_start("initial_strict_diagnostics");
            harness.open_file("live_edit.pl", UNDECLARED_SOURCE)?;

            let diagnostics = harness.wait_for_diagnostics("live_edit.pl", Duration::from_secs(6));
            let has_diagnostic = has_global_symbol_diagnostic(&diagnostics, "$name");
            if has_diagnostic {
                recorder.mark_first_useful_result("initial_strict_diagnostics");
            }
            recorder
                .check("strict diagnostics identify undeclared $name after open", has_diagnostic)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn given_live_edit_when_variable_is_declared_then_navigation_remains_responsive() {
    run_ux_scenario(
        "live_edit_feedback_loop",
        SCENARIO_FILE,
        "given_live_edit_when_variable_is_declared_then_navigation_remains_responsive",
        UxCiTier::Pr,
        Some(UxComponent::Diagnostics),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_live_edit_harness()?;
            recorder.mark_request_start("pre_edit_strict_diagnostics");
            harness.open_file("live_edit.pl", UNDECLARED_SOURCE)?;

            let before = harness.wait_for_diagnostics("live_edit.pl", Duration::from_secs(6));
            let precondition_met = has_global_symbol_diagnostic(&before, "$name");
            if precondition_met {
                recorder.mark_first_useful_result("pre_edit_strict_diagnostics");
            }
            recorder
                .check("pre-edit strict diagnostics identify undeclared $name", precondition_met)?;

            harness.change_file_full("live_edit.pl", DECLARED_SOURCE)?;

            recorder.mark_request_start("post_edit_diagnostics_refresh");
            let post_edit_diagnostics =
                harness.wait_for_latest_diagnostics("live_edit.pl", Duration::from_secs(6));
            recorder.mark_first_useful_result("post_edit_diagnostics_refresh");
            recorder.check(
                "post-edit diagnostics refresh completes with a valid diagnostics payload",
                post_edit_diagnostics.iter().all(|diag| diag.get("message").is_some()),
            )?;

            recorder.mark_request_start("post_edit_definition");
            let definitions = harness.definition("live_edit.pl", 4, 7);
            if definitions.is_ok() {
                recorder.mark_first_useful_result("post_edit_definition");
            }
            recorder.check(
                "go-to-definition remains responsive after didChange",
                definitions.is_ok(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
