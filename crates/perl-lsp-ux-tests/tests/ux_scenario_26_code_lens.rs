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

// Binary skip messages are visible only in integration-test output.
#![allow(clippy::print_stderr)]

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

const EXPECTED_REFERENCE_LENS_NAMES: &[&str] = &["MyCalc", "new", "add", "subtract", "multiply"];

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

fn module_harness() -> Result<UxHarness> {
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("mycalc.pl", CODELENS_FIXTURE))?;
    harness.open_file("mycalc.pl", CODELENS_FIXTURE)?;
    Ok(harness)
}

fn test_harness() -> Result<UxHarness> {
    let harness = UxHarness::new(ScenarioConfig::default().with_file("mytest.t", TEST_FIXTURE))?;
    harness.open_file("mytest.t", TEST_FIXTURE)?;
    Ok(harness)
}

fn request_code_lenses(harness: &UxHarness, path: &str) -> Result<Vec<Value>> {
    let uri = harness.workspace.uri(path);
    let response = harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    if let Some(error) = response.get("error") {
        anyhow::bail!("codeLens returned a JSON-RPC error: {error}");
    }

    response
        .get("result")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("codeLens result MUST be an array, got: {response:?}"))
}

fn lens_data_name(lens: &Value) -> Option<&str> {
    lens.pointer("/data/name").and_then(Value::as_str)
}

fn command_name(lens: &Value) -> Option<&str> {
    lens.pointer("/command/command").and_then(Value::as_str)
}

#[test]
fn scenario_26_code_lens_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_26: perl-lsp binary not found");
        return Ok(());
    }

    let harness = module_harness()?;
    let lenses = request_code_lenses(&harness, "mycalc.pl")?;

    assert!(
        !lenses.is_empty(),
        "codeLens on a module with a package and subroutines MUST return useful lenses"
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

    let harness = module_harness()?;
    let lenses = request_code_lenses(&harness, "mycalc.pl")?;

    assert!(
        !lenses.is_empty(),
        "codeLens result MUST be a non-empty array for a module with package/sub declarations"
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

    let harness = module_harness()?;
    let lenses = request_code_lenses(&harness, "mycalc.pl")?;

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
        assert!(range.get("end").is_some(), "CodeLens[{i}].range MUST have 'end', got: {range:?}");
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

    let harness = module_harness()?;

    // Two identical requests in sequence must succeed without crash.
    let mut previous_count = None;
    for round in 1u8..=2 {
        let lenses = request_code_lenses(&harness, "mycalc.pl")?;
        assert!(
            !lenses.is_empty(),
            "codeLens round {round} MUST return module lenses for a package/sub fixture"
        );
        if let Some(count) = previous_count {
            assert_eq!(lenses.len(), count, "codeLens MUST be idempotent for repeated requests");
        }
        previous_count = Some(lenses.len());
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

    let harness = test_harness()?;
    let lenses = request_code_lenses(&harness, "mytest.t")?;

    assert!(
        lenses.iter().any(|lens| command_name(lens) == Some("perl.runTestFile")),
        "codeLens on .t files MUST include a run-all-tests command, got: {lenses:?}"
    );
    assert!(
        lenses.iter().any(|lens| command_name(lens) == Some("perl.runTest")),
        "codeLens on named test subs MUST include run-test commands, got: {lenses:?}"
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

    let harness = module_harness()?;
    let lenses = request_code_lenses(&harness, "mycalc.pl")?;

    for expected in EXPECTED_REFERENCE_LENS_NAMES {
        assert!(
            lenses.iter().any(|lens| lens_data_name(lens) == Some(*expected)),
            "codeLens MUST include an unresolved reference lens for `{expected}`, got: {lenses:?}"
        );
    }

    let reference_lens = lenses
        .iter()
        .find(|lens| lens.get("data").is_some())
        .ok_or_else(|| anyhow::anyhow!("expected at least one unresolved reference lens"))?;
    let resolve_response =
        harness.client.request("codeLens/resolve", reference_lens.clone(), REQUEST_TIMEOUT)?;

    if let Some(error) = resolve_response.get("error") {
        anyhow::bail!("codeLens/resolve returned a JSON-RPC error: {error}");
    }
    let resolved = resolve_response
        .get("result")
        .ok_or_else(|| anyhow::anyhow!("codeLens/resolve must return a result"))?;
    assert_eq!(
        command_name(resolved),
        Some("editor.action.findReferences"),
        "codeLens/resolve MUST turn unresolved reference lenses into find-references commands"
    );

    harness.assert_no_crash();
    Ok(())
}
