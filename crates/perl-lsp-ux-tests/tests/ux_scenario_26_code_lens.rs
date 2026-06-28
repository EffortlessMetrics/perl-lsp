//! Scenario 26: Code lens UX coverage.
//!
//! Exercises `textDocument/codeLens` and `codeLens/resolve` against a typical
//! Perl module with named subroutines, package declarations, and test helpers.
//!
//! Contract:
//! - `codeLens` MUST NOT return a JSON-RPC error for any openable Perl file.
//! - `codeLens` MUST return an array (possibly empty), never a non-array result.
//! - Each returned CodeLens MUST have a `range` with `start` and `end` positions.
//! - `codeLens/resolve` on any lens returned by `codeLens` MUST NOT error.
//! - The server MUST stay stable after requesting lenses on the same file twice
//!   (idempotency guard).

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// A realistic OO-style Perl module with package, several subs, and a call
/// site and exercises the most common code-lens attachment points.
const CODELENS_FIXTURE: &str = r#"package MyCalc;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub add {
    my ($self, $x, $y) = @_;
    return $x + $y;
}

sub subtract {
    my ($self, $x, $y) = @_;
    return $x - $y;
}

sub multiply {
    my ($self, $x, $y) = @_;
    return $x * $y;
}

my $calc = MyCalc->new();
my $sum  = $calc->add(3, 4);
print "Result: $sum\n";
1;
"#;

/// A minimal test file so the code-lens provider can detect `Test::*` subs.
const TEST_FIXTURE: &str = r#"use strict;
use warnings;
use Test::More;

sub test_addition {
    is(2 + 2, 4, 'addition works');
}

sub test_subtraction {
    is(5 - 3, 2, 'subtraction works');
}

test_addition();
test_subtraction();
done_testing();
"#;

#[test]
fn scenario_26_code_lens_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("mycalc.pl", CODELENS_FIXTURE))?;
    harness.open_file("mycalc.pl", CODELENS_FIXTURE)?;
    let uri = harness.workspace.uri("mycalc.pl");

    let response = harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(
        response.get("error").is_none(),
        "codeLens MUST NOT return a JSON-RPC error, got: {:?}",
        response
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_26_code_lens_result_is_array() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("mycalc.pl", CODELENS_FIXTURE))?;
    harness.open_file("mycalc.pl", CODELENS_FIXTURE)?;
    let uri = harness.workspace.uri("mycalc.pl");

    let response = harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(response.get("error").is_none(), "codeLens returned error: {:?}", response);

    let result = response.get("result").unwrap_or(&Value::Null);
    assert!(
        result.is_array() || result.is_null(),
        "codeLens result MUST be an array (or null for no lenses), got: {result:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_26_code_lens_items_have_valid_ranges() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("mycalc.pl", CODELENS_FIXTURE))?;
    harness.open_file("mycalc.pl", CODELENS_FIXTURE)?;
    let uri = harness.workspace.uri("mycalc.pl");

    let response = harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(response.get("error").is_none(), "codeLens returned error: {:?}", response);

    let result = response.get("result").unwrap_or(&Value::Null);
    if let Some(lenses) = result.as_array() {
        for (i, lens) in lenses.iter().enumerate() {
            assert!(
                lens.get("range").is_some(),
                "CodeLens[{i}] MUST have a 'range' field, got: {lens:?}"
            );
            let Some(range) = lens.get("range") else { continue };
            assert!(
                range.get("start").is_some(),
                "CodeLens[{i}].range MUST have 'start', got: {range:?}"
            );
            assert!(
                range.get("end").is_some(),
                "CodeLens[{i}].range MUST have 'end', got: {range:?}"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_26_code_lens_is_idempotent() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("mycalc.pl", CODELENS_FIXTURE))?;
    harness.open_file("mycalc.pl", CODELENS_FIXTURE)?;
    let uri = harness.workspace.uri("mycalc.pl");

    // Two identical requests in sequence must succeed without crash.
    for round in 1u8..=2 {
        let response = harness.client.request(
            "textDocument/codeLens",
            json!({ "textDocument": { "uri": uri } }),
            REQUEST_TIMEOUT,
        )?;
        assert!(
            response.get("error").is_none(),
            "codeLens round {round} MUST NOT return error: {:?}",
            response
        );
    }

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_26_code_lens_on_test_file_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }
    let harness = UxHarness::new(ScenarioConfig::default().with_file("mytest.t", TEST_FIXTURE))?;
    harness.open_file("mytest.t", TEST_FIXTURE)?;
    let uri = harness.workspace.uri("mytest.t");

    let response = harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(
        response.get("error").is_none(),
        "codeLens on .t test file MUST NOT return JSON-RPC error: {:?}",
        response
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_26_code_lens_resolve_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("mycalc.pl", CODELENS_FIXTURE))?;
    harness.open_file("mycalc.pl", CODELENS_FIXTURE)?;
    let uri = harness.workspace.uri("mycalc.pl");

    // Fetch lenses first so we can resolve the first one (if any exist).
    let lens_response = harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;
    assert!(
        lens_response.get("error").is_none(),
        "codeLens initial fetch returned error: {:?}",
        lens_response
    );

    let lenses = match lens_response.get("result").and_then(Value::as_array) {
        Some(arr) if !arr.is_empty() => arr.clone(),
        // No lenses returned; skip resolve check (not an error).
        _ => {
            harness.assert_no_crash();
            return Ok(());
        }
    };

    let first_lens = &lenses[0];
    let resolve_response =
        harness.client.request("codeLens/resolve", first_lens.clone(), REQUEST_TIMEOUT)?;

    assert!(
        resolve_response.get("error").is_none(),
        "codeLens/resolve MUST NOT return JSON-RPC error: {:?}",
        resolve_response
    );

    harness.assert_no_crash();
    Ok(())
}
