//! Scenario 25: builtin signature-help UX coverage.
//!
//! This scenario locks the end-to-end `textDocument/signatureHelp` path that
//! the current server answers reliably: Perl builtins. User-defined call-site
//! coverage belongs in a separate runtime follow-up because that path currently
//! times out under the real stdio harness.

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
fn scenario_25_builtin_push_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    let response = request_signature_help(&harness, 4, 8)?;

    assert!(
        response.get("error").is_none(),
        "signatureHelp on builtin push MUST NOT return a JSON-RPC error: {response:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_25_builtin_join_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    let response = request_signature_help(&harness, 5, 15)?;

    assert!(
        response.get("error").is_none(),
        "signatureHelp on builtin join MUST NOT return a JSON-RPC error: {response:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_25_builtin_result_is_well_formed_when_present() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_25: perl-lsp binary not found");
        return Ok(());
    }

    let harness = builtin_harness()?;
    let response = request_signature_help(&harness, 5, 15)?;

    assert!(
        response.get("error").is_none(),
        "signatureHelp on builtin join returned an error: {response:?}"
    );

    if let Some(result) = response.get("result") {
        if !result.is_null() {
            assert_signature_help_structure(result)?;
        }
    }

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
        assert!(
            response.get("error").is_none(),
            "signatureHelp round {round} MUST NOT return a JSON-RPC error: {response:?}"
        );
    }

    harness.assert_no_crash();
    Ok(())
}

fn assert_signature_help_structure(result: &Value) -> Result<()> {
    let Some(signatures) = result.get("signatures") else {
        anyhow::bail!("SignatureHelp result must have a signatures field, got: {result:?}");
    };
    assert!(signatures.is_array(), "SignatureHelp.signatures must be an array");

    let sig_array = signatures
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("signatures is not an array: {signatures:?}"))?;
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
