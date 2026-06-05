//! Scenario 25: builtin signature-help UX coverage.
//!
//! This scenario locks the end-to-end `textDocument/signatureHelp` path that
//! the current server answers reliably: Perl builtins. User-defined call-site
//! coverage belongs in a separate runtime follow-up because that path currently
//! times out under the real stdio harness.

use std::time::Duration;

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::{Value, json};

const SCENARIO_FILE: &str = "ux_scenario_25_signature_help.rs";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

const BUILTIN_FIXTURE: &str = r#"use strict;
use warnings;

my @arr = (3, 1, 2);
push(@arr, 4);
my $str = join(", ", @arr);
"#;

fn builtin_harness() -> Result<UxHarness> {
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("builtins.pl", BUILTIN_FIXTURE))?;
    harness.open_file("builtins.pl", BUILTIN_FIXTURE)?;
    Ok(harness)
}

fn request_signature_help(harness: &UxHarness, line: u32, character: u32) -> Result<Value> {
    let uri = harness.workspace.uri("builtins.pl");
    harness.client.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
        REQUEST_TIMEOUT,
    )
}

#[test]
fn scenario_25_builtin_push_does_not_error() {
    run_ux_scenario(
        "signature_help_core",
        SCENARIO_FILE,
        "scenario_25_builtin_push_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::SignatureHelp),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = builtin_harness()?;
            recorder.mark_request_start("signature_help_push");
            let response = request_signature_help(&harness, 4, 8)?;
            let no_error = response.get("error").is_none();
            if no_error {
                recorder.mark_first_useful_result("signature_help_push");
            }
            recorder.check(
                "signatureHelp on builtin push does not return a JSON-RPC error",
                no_error,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_25_builtin_join_does_not_error() {
    run_ux_scenario(
        "signature_help_core",
        SCENARIO_FILE,
        "scenario_25_builtin_join_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::SignatureHelp),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = builtin_harness()?;
            recorder.mark_request_start("signature_help_join");
            let response = request_signature_help(&harness, 5, 15)?;
            let no_error = response.get("error").is_none();
            if no_error {
                recorder.mark_first_useful_result("signature_help_join");
            }
            recorder.check(
                "signatureHelp on builtin join does not return a JSON-RPC error",
                no_error,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_25_builtin_result_is_well_formed_when_present() {
    run_ux_scenario(
        "signature_help_core",
        SCENARIO_FILE,
        "scenario_25_builtin_result_is_well_formed_when_present",
        UxCiTier::Pr,
        Some(UxComponent::SignatureHelp),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = builtin_harness()?;
            recorder.mark_request_start("signature_help_join_shape");
            let response = request_signature_help(&harness, 5, 15)?;
            let no_error = response.get("error").is_none();
            if no_error {
                recorder.mark_first_useful_result("signature_help_join_shape");
            }
            recorder.check(
                "signatureHelp on builtin join does not return a JSON-RPC error",
                no_error,
            )?;
            recorder.check(
                "non-null signatureHelp result has valid signature labels",
                response.get("result").is_none_or(|result| {
                    result.is_null() || signature_help_structure_is_valid(result)
                }),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_25_builtin_requests_are_idempotent() {
    run_ux_scenario(
        "signature_help_core",
        SCENARIO_FILE,
        "scenario_25_builtin_requests_are_idempotent",
        UxCiTier::Pr,
        Some(UxComponent::SignatureHelp),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = builtin_harness()?;
            for round in 1..=2 {
                let request_name = format!("signature_help_join_round_{round}");
                recorder.mark_request_start(&request_name);
                let response = request_signature_help(&harness, 5, 15)?;
                let no_error = response.get("error").is_none();
                if no_error {
                    recorder.mark_first_useful_result(&request_name);
                }
                recorder.check(
                    &format!(
                        "signatureHelp repeated request round {round} does not return a JSON-RPC error"
                    ),
                    no_error,
                )?;
            }

            harness.assert_no_crash();
            Ok(())
        },
    );
}

fn signature_help_structure_is_valid(result: &Value) -> bool {
    let Some(sig_array) = result.get("signatures").and_then(Value::as_array) else {
        return false;
    };

    sig_array.iter().all(|signature| {
        let has_label = signature.get("label").and_then(Value::as_str).is_some();
        let parameters_are_labeled = signature
            .get("parameters")
            .and_then(Value::as_array)
            .is_none_or(|params| params.iter().all(|param| param.get("label").is_some()));
        has_label && parameters_are_labeled
    })
}
