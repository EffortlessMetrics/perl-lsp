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
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};
use std::time::Duration;

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

#[test]
fn scenario_27_folding_range_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("processor.pl", FOLDING_FIXTURE))?;
    harness.open_file("processor.pl", FOLDING_FIXTURE)?;
    let uri = harness.workspace.uri("processor.pl");

    let response = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(
        response.get("error").is_none(),
        "foldingRange MUST NOT return a JSON-RPC error, got: {:?}",
        response
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_27_folding_range_result_is_array() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("processor.pl", FOLDING_FIXTURE))?;
    harness.open_file("processor.pl", FOLDING_FIXTURE)?;
    let uri = harness.workspace.uri("processor.pl");

    let response = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(response.get("error").is_none(), "foldingRange returned error: {:?}", response);

    let result = response.get("result").unwrap_or(&Value::Null);
    assert!(
        result.is_array() || result.is_null(),
        "foldingRange result MUST be an array (or null), got: {result:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_27_folding_ranges_have_valid_line_fields() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("processor.pl", FOLDING_FIXTURE))?;
    harness.open_file("processor.pl", FOLDING_FIXTURE)?;
    let uri = harness.workspace.uri("processor.pl");

    let response = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(response.get("error").is_none(), "foldingRange returned error: {:?}", response);

    let result = response.get("result").unwrap_or(&Value::Null);
    if let Some(ranges) = result.as_array() {
        for (i, range) in ranges.iter().enumerate() {
            let start_line = range.get("startLine").and_then(Value::as_u64).ok_or_else(|| {
                anyhow::anyhow!("FoldingRange[{i}] MUST have integer 'startLine', got: {range:?}")
            })?;
            let end_line = range.get("endLine").and_then(Value::as_u64).ok_or_else(|| {
                anyhow::anyhow!("FoldingRange[{i}] MUST have integer 'endLine', got: {range:?}")
            })?;
            assert!(
                start_line <= end_line,
                "FoldingRange[{i}] startLine ({start_line}) MUST be <= endLine ({end_line})"
            );
        }
    }

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_27_multi_block_file_produces_at_least_one_fold() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("processor.pl", FOLDING_FIXTURE))?;
    harness.open_file("processor.pl", FOLDING_FIXTURE)?;
    let uri = harness.workspace.uri("processor.pl");

    let response = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(response.get("error").is_none(), "foldingRange returned error: {:?}", response);

    let result = response.get("result").unwrap_or(&Value::Null);
    let range_count = result.as_array().map_or(0, |v| v.len());
    assert!(
        range_count >= 1,
        "A file with multiple sub/if/for/while blocks MUST produce at least one folding range; got {range_count}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_27_empty_file_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness = UxHarness::new(ScenarioConfig::default().with_file("empty.pl", EMPTY_FIXTURE))?;
    harness.open_file("empty.pl", EMPTY_FIXTURE)?;
    let uri = harness.workspace.uri("empty.pl");

    let response = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(
        response.get("error").is_none(),
        "foldingRange on empty file MUST NOT return JSON-RPC error: {:?}",
        response
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_27_single_line_file_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("tiny.pl", SINGLE_LINE_FIXTURE))?;
    harness.open_file("tiny.pl", SINGLE_LINE_FIXTURE)?;
    let uri = harness.workspace.uri("tiny.pl");

    let response = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(
        response.get("error").is_none(),
        "foldingRange on single-line file MUST NOT return JSON-RPC error: {:?}",
        response
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_27_folding_range_is_idempotent() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_27: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("processor.pl", FOLDING_FIXTURE))?;
    harness.open_file("processor.pl", FOLDING_FIXTURE)?;
    let uri = harness.workspace.uri("processor.pl");

    // Request twice; both requests must succeed without crashing.
    let first = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;
    let second = harness.client.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": uri } }),
        REQUEST_TIMEOUT,
    )?;

    assert!(first.get("error").is_none(), "foldingRange first request errored: {:?}", first);
    assert!(second.get("error").is_none(), "foldingRange second request errored: {:?}", second);

    let first_count = first.get("result").and_then(Value::as_array).map_or(0, |v| v.len());
    let second_count = second.get("result").and_then(Value::as_array).map_or(0, |v| v.len());

    assert_eq!(
        first_count, second_count,
        "foldingRange MUST be idempotent: first={first_count}, second={second_count}"
    );

    harness.assert_no_crash();
    Ok(())
}
