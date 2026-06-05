//! Scenario 18 — Go-to-declaration feature grid coverage.
//!
//! Verifies that `textDocument/declaration` is wired up end-to-end for the LSP
//! server process used in UX regression testing.
//!
//! Contract:
//! - `textDocument/declaration` MUST NOT return a JSON-RPC error.
//! - A declaration result MAY be empty (degraded mode acceptable) but must not crash.
//! - When non-empty, each result MUST include URI/range shape (`targetUri` +
//!   `targetRange` for links, or `uri` + `range` for locations).

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;

const SCENARIO_FILE: &str = "ux_scenario_18_goto_declaration.rs";

const DECLARATION_FIXTURE: &str = r#"use strict;
use warnings;

my $value = 41;

sub inc {
    my ($n) = @_;
    return $n + 1;
}

my $result = inc($value);
print "$result\n";
"#;

fn is_declaration_location_shape(entry: &Value) -> bool {
    let is_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    is_link || is_location
}

#[test]
fn scenario_18_declaration_request_does_not_error() {
    run_ux_scenario(
        "goto_declaration_core",
        SCENARIO_FILE,
        "scenario_18_declaration_request_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::GotoDefinition),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig::default().with_file("declaration.pl", DECLARATION_FIXTURE),
            )?;

            harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;
            recorder.mark_request_start("declaration_request");
            let result = harness.declaration("declaration.pl", 9, 13);
            if result.is_ok() {
                recorder.mark_first_useful_result("declaration_request");
            }

            recorder.check(
                "textDocument/declaration does not return a JSON-RPC error",
                result.is_ok(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_18_declaration_result_is_location_or_empty() {
    run_ux_scenario(
        "goto_declaration_core",
        SCENARIO_FILE,
        "scenario_18_declaration_result_is_location_or_empty",
        UxCiTier::Pr,
        Some(UxComponent::GotoDefinition),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig::default().with_file("declaration.pl", DECLARATION_FIXTURE),
            )?;

            harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;
            recorder.mark_request_start("declaration_shape");
            let declarations = harness.declaration("declaration.pl", 9, 13)?;
            recorder.mark_first_useful_result("declaration_shape");

            recorder.check(
                "declaration result is clean empty or valid Location/LocationLink shape",
                declarations.iter().all(is_declaration_location_shape),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
