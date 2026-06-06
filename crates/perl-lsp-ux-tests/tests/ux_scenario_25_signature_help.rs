//! Scenario 25: builtin signature-help UX coverage.
//!
//! This scenario locks the end-to-end `textDocument/signatureHelp` path that
//! the current server answers reliably: Perl builtins. User-defined call-site
//! coverage belongs in a separate runtime follow-up because that path currently
//! times out under the real stdio harness.

// Binary skip messages are visible only in integration-test output.
#![allow(clippy::print_stderr)]

use std::time::Duration;

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};

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
fn scenario_25_builtin_push_returns_signature_label() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    let response = request_signature_help(&harness, 4, 8)?;

    assert_builtin_signature_label(&response, "push ARRAY, LIST")?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_25_builtin_join_returns_signature_label() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    let response = request_signature_help(&harness, 5, 15)?;

    assert_builtin_signature_label(&response, "join EXPR, LIST")?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_25_builtin_result_is_well_formed() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    let response = request_signature_help(&harness, 5, 15)?;

    let result = non_null_signature_help_result(&response)?;
    assert_signature_help_structure(result)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_25_builtin_requests_are_idempotent() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    for round in 1..=2 {
        let response = request_signature_help(&harness, 5, 15)?;
        assert_builtin_signature_label(&response, "join EXPR, LIST")
            .map_err(|error| anyhow::anyhow!("signatureHelp round {round}: {error}"))?;
    }

    harness.assert_no_crash();
    Ok(())
}

fn assert_builtin_signature_label(response: &Value, expected_label: &str) -> Result<()> {
    let result = non_null_signature_help_result(response)?;
    assert_signature_help_structure(result)?;
    let labels = signature_labels(result)?;
    assert!(
        labels.contains(&expected_label),
        "SignatureHelp must include builtin label `{expected_label}`, got: {labels:?}"
    );
    Ok(())
}

fn non_null_signature_help_result(response: &Value) -> Result<&Value> {
    if let Some(error) = response.get("error") {
        anyhow::bail!("signatureHelp returned a JSON-RPC error: {error}");
    }

    let Some(result) = response.get("result") else {
        anyhow::bail!("signatureHelp response must include result, got: {response:?}");
    };
    if result.is_null() {
        anyhow::bail!("signatureHelp result must be non-null for builtin call sites");
    }
    Ok(result)
}

fn signature_labels(result: &Value) -> Result<Vec<&str>> {
    let signatures = result
        .get("signatures")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("SignatureHelp.signatures must be an array"))?;
    Ok(signatures.iter().filter_map(|sig| sig.get("label").and_then(Value::as_str)).collect())
}

fn assert_signature_help_structure(result: &Value) -> Result<()> {
    let Some(signatures) = result.get("signatures") else {
        anyhow::bail!("SignatureHelp result must have a signatures field, got: {result:?}");
    };
    assert!(signatures.is_array(), "SignatureHelp.signatures must be an array");

    let sig_array = signatures
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("signatures is not an array: {signatures:?}"))?;
    assert!(!sig_array.is_empty(), "SignatureHelp.signatures must not be empty");
    for (i, sig) in sig_array.iter().enumerate() {
        assert!(
            sig.get("label").and_then(Value::as_str).is_some(),
            "SignatureInformation[{i}] must have a string label, got: {sig:?}"
        );

        if let Some(params) = sig.get("parameters").and_then(Value::as_array) {
            for (j, param) in params.iter().enumerate() {
                assert!(
                    param.get("label").is_some(),
                    "ParameterInformation[{i}][{j}] must have a label, got: {param:?}"
                );
            }
        }
    }
    Ok(())
}
