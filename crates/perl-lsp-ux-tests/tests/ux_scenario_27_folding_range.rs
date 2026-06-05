//! Scenario 27: Folding range UX coverage.
//!
//! Exercises `textDocument/foldingRange` against Perl files with nested
//! blocks, the primary use-case for code folding in an editor.
//!
//! Contract:
//! - `foldingRange` MUST NOT return a JSON-RPC error for any valid Perl file.
//! - If ranges are returned, each MUST have `startLine` and `endLine` integers,
//!   with `startLine <= endLine`.
//! - An empty file MUST return an empty array (not an error).
//! - A file with deeply nested blocks (if/else/for/while/sub) MUST return
//!   at least one range so the editor can offer at least one fold.
//! - The server MUST remain stable after repeated fold requests.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_27_folding_range.rs";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Multi-block Perl file that exercises every common fold anchor:
/// sub body, if/elsif/else, for loop, while loop, and a nested closure.
const FOLDING_FIXTURE: &str = r#"package MyProcessor;
use strict;
use warnings;

sub process_items {
    my ($self, @items) = @_;
    my @results;

    for my $item (@items) {
        if ($item > 0) {
            push @results, $item * 2;
        } elsif ($item == 0) {
            push @results, 0;
        } else {
            push @results, -1;
        }
    }

    return @results;
}

sub run_while_loop {
    my ($self, $limit) = @_;
    my $count = 0;
    while ($count < $limit) {
        $count++;
    }
    return $count;
}

sub with_closure {
    my ($self) = @_;
    my $adder = sub {
        my ($x) = @_;
        return $x + 10;
    };
    return $adder;
}

1;
"#;

const EMPTY_FIXTURE: &str = "";

const SINGLE_LINE_FIXTURE: &str = "my $x = 42;\n";

fn open_fixture(file: &str, source: &str) -> Result<UxHarness> {
    let harness = UxHarness::new(ScenarioConfig::default().with_file(file, source))?;
    harness.open_file(file, source)?;
    Ok(harness)
}

fn request_folding_range(harness: &UxHarness, file: &str) -> Result<Value> {
    let uri = harness.workspace.uri(file);
    harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )
}

fn response_has_no_error(response: &Value) -> bool {
    response.get("error").is_none()
}

fn folding_result_is_array_or_null(response: &Value) -> bool {
    response.get("result").is_none_or(|result| result.is_array() || result.is_null())
}

fn folding_range_has_valid_lines(range: &Value) -> bool {
    let Some(start_line) = range.get("startLine").and_then(Value::as_u64) else {
        return false;
    };
    let Some(end_line) = range.get("endLine").and_then(Value::as_u64) else {
        return false;
    };
    start_line <= end_line
}

fn folding_ranges_have_valid_lines(response: &Value) -> bool {
    response.get("result").is_none_or(|result| {
        result.is_null()
            || result
                .as_array()
                .is_some_and(|ranges| ranges.iter().all(folding_range_has_valid_lines))
    })
}

fn folding_range_count(response: &Value) -> usize {
    response.get("result").and_then(Value::as_array).map_or(0, Vec::len)
}

#[test]
fn scenario_27_folding_range_does_not_error() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_folding_range_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("processor.pl", FOLDING_FIXTURE)?;
            recorder.mark_request_start("folding_range_processor");
            let response = request_folding_range(&harness, "processor.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error {
                recorder.mark_first_useful_result("folding_range_processor");
            }
            recorder.check("foldingRange does not return a JSON-RPC error", no_error)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_27_folding_range_result_is_array() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_folding_range_result_is_array",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("processor.pl", FOLDING_FIXTURE)?;
            recorder.mark_request_start("folding_range_result_shape");
            let response = request_folding_range(&harness, "processor.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error && folding_result_is_array_or_null(&response) {
                recorder.mark_first_useful_result("folding_range_result_shape");
            }
            recorder.check("foldingRange does not return a JSON-RPC error", no_error)?;
            recorder.check(
                "foldingRange result is an array or null",
                folding_result_is_array_or_null(&response),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_27_folding_ranges_have_valid_line_fields() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_folding_ranges_have_valid_line_fields",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("processor.pl", FOLDING_FIXTURE)?;
            recorder.mark_request_start("folding_range_line_fields");
            let response = request_folding_range(&harness, "processor.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error && folding_ranges_have_valid_lines(&response) {
                recorder.mark_first_useful_result("folding_range_line_fields");
            }
            recorder.check("foldingRange does not return a JSON-RPC error", no_error)?;
            recorder.check(
                "folding ranges include valid startLine and endLine fields when present",
                folding_ranges_have_valid_lines(&response),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_27_multi_block_file_produces_at_least_one_fold() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_multi_block_file_produces_at_least_one_fold",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("processor.pl", FOLDING_FIXTURE)?;
            recorder.mark_request_start("folding_range_nonempty_blocks");
            let response = request_folding_range(&harness, "processor.pl")?;
            let range_count = folding_range_count(&response);
            if response_has_no_error(&response) && range_count >= 1 {
                recorder.mark_first_useful_result("folding_range_nonempty_blocks");
            }
            recorder.check(
                "foldingRange does not return a JSON-RPC error",
                response_has_no_error(&response),
            )?;
            recorder
                .check("multi-block file produces at least one folding range", range_count >= 1)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_27_empty_file_does_not_error() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_empty_file_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("empty.pl", EMPTY_FIXTURE)?;
            recorder.mark_request_start("folding_range_empty_file");
            let response = request_folding_range(&harness, "empty.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error {
                recorder.mark_first_useful_result("folding_range_empty_file");
            }
            recorder
                .check("foldingRange on empty file does not return a JSON-RPC error", no_error)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_27_single_line_file_does_not_error() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_single_line_file_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("tiny.pl", SINGLE_LINE_FIXTURE)?;
            recorder.mark_request_start("folding_range_single_line");
            let response = request_folding_range(&harness, "tiny.pl")?;
            let no_error = response_has_no_error(&response);
            if no_error {
                recorder.mark_first_useful_result("folding_range_single_line");
            }
            recorder.check(
                "foldingRange on single-line file does not return a JSON-RPC error",
                no_error,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_27_folding_range_is_idempotent() {
    run_ux_scenario(
        "folding_range_core",
        SCENARIO_FILE,
        "scenario_27_folding_range_is_idempotent",
        UxCiTier::Pr,
        Some(UxComponent::FoldingRange),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = open_fixture("processor.pl", FOLDING_FIXTURE)?;
            recorder.mark_request_start("folding_range_first");
            let first = request_folding_range(&harness, "processor.pl")?;
            if response_has_no_error(&first) {
                recorder.mark_first_useful_result("folding_range_first");
            }
            recorder.check(
                "foldingRange first request does not return a JSON-RPC error",
                response_has_no_error(&first),
            )?;

            recorder.mark_request_start("folding_range_second");
            let second = request_folding_range(&harness, "processor.pl")?;
            if response_has_no_error(&second) {
                recorder.mark_first_useful_result("folding_range_second");
            }
            recorder.check(
                "foldingRange second request does not return a JSON-RPC error",
                response_has_no_error(&second),
            )?;

            recorder.check(
                "foldingRange repeated requests return the same range count",
                folding_range_count(&first) == folding_range_count(&second),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
