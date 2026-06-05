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
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_26_code_lens.rs";
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

fn open_fixture(file: &str, source: &str) -> Result<UxHarness> {
    let harness = UxHarness::new(ScenarioConfig::default().with_file(file, source))?;
    harness.open_file(file, source)?;
    Ok(harness)
}

fn request_code_lens(harness: &UxHarness, file: &str) -> Result<Value> {
    let uri = harness.workspace.uri(file);
    harness.client.request(
        "textDocument/codeLens",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )
}

fn response_has_no_error(response: &Value) -> bool {
    response.get("error").is_none()
}

fn code_lens_result_is_array_or_null(response: &Value) -> bool {
    response.get("result").is_none_or(|result| result.is_array() || result.is_null())
}

fn lens_has_valid_range(lens: &Value) -> bool {
    lens.get("range")
        .is_some_and(|range| range.get("start").is_some() && range.get("end").is_some())
}

fn code_lens_ranges_are_valid(response: &Value) -> bool {
    response.get("result").is_none_or(|result| {
        result.is_null()
            || result.as_array().is_some_and(|lenses| lenses.iter().all(lens_has_valid_range))
    })
}

#[test]
fn scenario_26_code_lens_does_not_error() {
    run_ux_scenario(
        "code_lens_core",
        SCENARIO_FILE,
        "scenario_26_code_lens_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::CodeLens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("mycalc.pl", CODELENS_FIXTURE)?;
            recorder.mark_request_start("code_lens_module");
            let response = request_code_lens(&harness, "mycalc.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error {
                recorder.mark_first_useful_result("code_lens_module");
            }
            recorder.check("codeLens does not return a JSON-RPC error", no_error)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_26_code_lens_result_is_array() {
    run_ux_scenario(
        "code_lens_core",
        SCENARIO_FILE,
        "scenario_26_code_lens_result_is_array",
        UxCiTier::Pr,
        Some(UxComponent::CodeLens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("mycalc.pl", CODELENS_FIXTURE)?;
            recorder.mark_request_start("code_lens_result_shape");
            let response = request_code_lens(&harness, "mycalc.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error && code_lens_result_is_array_or_null(&response) {
                recorder.mark_first_useful_result("code_lens_result_shape");
            }
            recorder.check("codeLens does not return a JSON-RPC error", no_error)?;
            recorder.check(
                "codeLens result is an array or null",
                code_lens_result_is_array_or_null(&response),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_26_code_lens_items_have_valid_ranges() {
    run_ux_scenario(
        "code_lens_core",
        SCENARIO_FILE,
        "scenario_26_code_lens_items_have_valid_ranges",
        UxCiTier::Pr,
        Some(UxComponent::CodeLens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("mycalc.pl", CODELENS_FIXTURE)?;
            recorder.mark_request_start("code_lens_ranges");
            let response = request_code_lens(&harness, "mycalc.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error && code_lens_ranges_are_valid(&response) {
                recorder.mark_first_useful_result("code_lens_ranges");
            }
            recorder.check("codeLens does not return a JSON-RPC error", no_error)?;
            recorder.check(
                "returned codeLens items have range start and end when present",
                code_lens_ranges_are_valid(&response),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_26_code_lens_is_idempotent() {
    run_ux_scenario(
        "code_lens_core",
        SCENARIO_FILE,
        "scenario_26_code_lens_is_idempotent",
        UxCiTier::Pr,
        Some(UxComponent::CodeLens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("mycalc.pl", CODELENS_FIXTURE)?;
            for round in 1u8..=2 {
                let request_name = format!("code_lens_module_round_{round}");
                recorder.mark_request_start(&request_name);
                let response = request_code_lens(&harness, "mycalc.pl")?;
                let no_error = response_has_no_error(&response);
                if no_error {
                    recorder.mark_first_useful_result(&request_name);
                }
                recorder.check(
                    &format!("codeLens repeated request round {round} does not return an error"),
                    no_error,
                )?;
            }

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_26_code_lens_on_test_file_does_not_error() {
    run_ux_scenario(
        "code_lens_core",
        SCENARIO_FILE,
        "scenario_26_code_lens_on_test_file_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::CodeLens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("mytest.t", TEST_FIXTURE)?;
            recorder.mark_request_start("code_lens_test_file");
            let response = request_code_lens(&harness, "mytest.t")?;
            let no_error = response_has_no_error(&response);
            if no_error {
                recorder.mark_first_useful_result("code_lens_test_file");
            }
            recorder
                .check("codeLens on .t test file does not return a JSON-RPC error", no_error)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_26_code_lens_resolve_does_not_error() {
    run_ux_scenario(
        "code_lens_core",
        SCENARIO_FILE,
        "scenario_26_code_lens_resolve_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::CodeLens),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("mycalc.pl", CODELENS_FIXTURE)?;
            recorder.mark_request_start("code_lens_resolve_fetch");
            let lens_response = request_code_lens(&harness, "mycalc.pl")?;
            let fetch_no_error = response_has_no_error(&lens_response);
            if fetch_no_error {
                recorder.mark_first_useful_result("code_lens_resolve_fetch");
            }
            recorder.check("codeLens initial fetch does not return an error", fetch_no_error)?;

            let Some(first_lens) = lens_response
                .get("result")
                .and_then(Value::as_array)
                .and_then(|lenses| lenses.first())
            else {
                harness.assert_no_crash();
                return Ok(());
            };

            recorder.mark_request_start("code_lens_resolve");
            let resolve_response =
                harness.client.request("codeLens/resolve", first_lens.clone(), REQUEST_TIMEOUT)?;
            let resolve_no_error = response_has_no_error(&resolve_response);
            if resolve_no_error {
                recorder.mark_first_useful_result("code_lens_resolve");
            }
            recorder
                .check("codeLens/resolve does not return a JSON-RPC error", resolve_no_error)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
