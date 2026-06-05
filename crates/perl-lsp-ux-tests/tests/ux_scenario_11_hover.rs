// Test infrastructure needs skip/status messages when the external binary is absent.
#![allow(clippy::print_stderr)]
// Test assertions intentionally panic with UX-specific failure messages.
#![allow(clippy::panic)]

//! Scenario 11 — Hover feature grid coverage.
//!
//! Verifies that `textDocument/hover` is wired up end-to-end for the LSP
//! feature advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - `textDocument/hover` MUST NOT return a JSON-RPC error.
//! - Hover over a known lexical variable MUST return a contents payload.
//! - When a result is returned it MUST have either `contents` (MarkupContent or
//!   MarkedString) and optionally a `range`.
//! - No crash signatures after the request.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

/// Perl source with a clearly-named sub and variable for hover targets.
const HOVER_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub calculate_sum {\n\
    my ($a, $b) = @_;\n\
    return $a + $b;\n\
}\n\
\n\
my $result = calculate_sum(3, 7);\n\
print $result;\n\
";

#[test]
fn scenario_11_hover_on_variable_returns_contents() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_11: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("calc.pl", HOVER_SOURCE),
    )?;

    harness.open_file("calc.pl", HOVER_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    // Hover on `$result` — line 8, char 3 (inside `$result`).
    let hover_result = harness.hover("calc.pl", 8, 3)?;
    let result = hover_result.ok_or_else(|| {
        anyhow::anyhow!("expected hover on known lexical $result to return contents")
    })?;
    assert_hover_contents_shape(&result);

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_11_hover_result_has_valid_shape() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_11: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("calc.pl", HOVER_SOURCE),
    )?;

    harness.open_file("calc.pl", HOVER_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    let result = harness.hover("calc.pl", 8, 3)?.ok_or_else(|| {
        anyhow::anyhow!("expected hover on known lexical $result to return contents")
    })?;
    assert_hover_contents_shape(&result);

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_11_hover_on_sub_name_does_not_crash() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_11: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("calc.pl", HOVER_SOURCE),
    )?;

    harness.open_file("calc.pl", HOVER_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    // Hover on `calculate_sum` sub declaration — line 3, char 4.
    harness.hover("calc.pl", 3, 4)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_11_hover_range_contains_cursor_when_present() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_11: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("calc.pl", HOVER_SOURCE),
    )?;

    harness.open_file("calc.pl", HOVER_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    // Hover on function call site `calculate_sum` — line 8, char 14.
    if let Some(result) = harness.hover("calc.pl", 8, 14)?
        && let Some(range) = result.get("range")
    {
        let start_line = range["start"]["line"].as_u64();
        let start_char = range["start"]["character"].as_u64();
        let end_line = range["end"]["line"].as_u64();
        let end_char = range["end"]["character"].as_u64();

        assert!(start_line.is_some(), "Hover range.start.line must be numeric");
        assert!(start_char.is_some(), "Hover range.start.character must be numeric");
        assert!(end_line.is_some(), "Hover range.end.line must be numeric");
        assert!(end_char.is_some(), "Hover range.end.character must be numeric");

        let (start_line, start_char, end_line, end_char) = (
            start_line.unwrap_or_default(),
            start_char.unwrap_or_default(),
            end_line.unwrap_or_default(),
            end_char.unwrap_or_default(),
        );

        assert!(start_line <= end_line, "Hover range start line must be <= end line: {:?}", range);
        if start_line == end_line {
            assert!(
                start_char <= end_char,
                "Hover range start char must be <= end char on same line: {:?}",
                range
            );
        }

        let cursor_line: u64 = 8;
        let cursor_char: u64 = 14;
        let starts_before_cursor =
            start_line < cursor_line || (start_line == cursor_line && start_char <= cursor_char);
        let ends_after_cursor =
            end_line > cursor_line || (end_line == cursor_line && end_char >= cursor_char);
        assert!(
            starts_before_cursor && ends_after_cursor,
            "Hover range should contain the cursor when provided: range={:?}, cursor=({}, {})",
            range,
            cursor_line,
            cursor_char
        );
    }

    harness.assert_no_crash();
    Ok(())
}

fn assert_hover_contents_shape(result: &Value) {
    assert!(
        result.get("contents").is_some(),
        "Hover result must have 'contents' field, got: {result:?}"
    );
    let contents = &result["contents"];
    let is_valid = contents.get("value").is_some()
        || contents.get("kind").is_some()
        || contents.is_string()
        || contents.is_array();
    assert!(
        is_valid,
        "Hover 'contents' must be MarkupContent, MarkedString, or array; got: {contents:?}"
    );
}
