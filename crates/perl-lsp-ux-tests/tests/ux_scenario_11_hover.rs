//! Scenario 11 — Hover feature grid coverage.
//!
//! Verifies that `textDocument/hover` is wired up end-to-end for the LSP
//! feature advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - `textDocument/hover` MUST NOT return a JSON-RPC error.
//! - When a result is returned it MUST have either `contents` (MarkupContent or
//!   MarkedString) and optionally a `range`.
//! - A null/empty result is acceptable (degraded mode).
//! - No crash signatures after the request.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_11_hover.rs";

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

fn create_hover_harness() -> Result<UxHarness> {
    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("calc.pl", HOVER_SOURCE),
    )?;
    harness.open_file("calc.pl", HOVER_SOURCE)?;
    std::thread::sleep(Duration::from_millis(300));
    Ok(harness)
}

fn hover_contents_has_valid_shape(result: &Value) -> bool {
    let Some(contents) = result.get("contents") else {
        return false;
    };
    contents.get("value").is_some()
        || contents.get("kind").is_some()
        || contents.is_string()
        || contents.is_array()
}

fn numeric_range(range: &Value) -> Option<(u64, u64, u64, u64)> {
    Some((
        range.pointer("/start/line")?.as_u64()?,
        range.pointer("/start/character")?.as_u64()?,
        range.pointer("/end/line")?.as_u64()?,
        range.pointer("/end/character")?.as_u64()?,
    ))
}

fn hover_range_contains_cursor_when_present(
    result: &Value,
    cursor_line: u64,
    cursor_char: u64,
) -> bool {
    let Some(range) = result.get("range") else {
        return true;
    };
    let Some((start_line, start_char, end_line, end_char)) = numeric_range(range) else {
        return false;
    };
    if start_line > end_line || (start_line == end_line && start_char > end_char) {
        return false;
    }

    let starts_before_cursor =
        start_line < cursor_line || (start_line == cursor_line && start_char <= cursor_char);
    let ends_after_cursor =
        end_line > cursor_line || (end_line == cursor_line && end_char >= cursor_char);
    starts_before_cursor && ends_after_cursor
}

#[test]
fn scenario_11_hover_on_variable_does_not_error() {
    run_ux_scenario(
        "hover_core",
        SCENARIO_FILE,
        "scenario_11_hover_on_variable_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::Hover),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_hover_harness()?;
            recorder.mark_request_start("hover_variable");
            let hover_result = harness.hover("calc.pl", 8, 3);
            if hover_result.is_ok() {
                recorder.mark_first_useful_result("hover_variable");
            }
            recorder.check(
                "textDocument/hover on variable does not return a JSON-RPC error",
                hover_result.is_ok(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_11_hover_result_has_valid_shape_when_non_null() {
    run_ux_scenario(
        "hover_core",
        SCENARIO_FILE,
        "scenario_11_hover_result_has_valid_shape_when_non_null",
        UxCiTier::Pr,
        Some(UxComponent::Hover),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_hover_harness()?;
            recorder.mark_request_start("hover_variable_shape");
            let hover_result = harness.hover("calc.pl", 8, 3)?;
            recorder.mark_first_useful_result("hover_variable_shape");
            recorder.check(
                "hover result is clean empty or has valid contents shape",
                hover_result.as_ref().is_none_or(hover_contents_has_valid_shape),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_11_hover_on_sub_name_does_not_crash() {
    run_ux_scenario(
        "hover_core",
        SCENARIO_FILE,
        "scenario_11_hover_on_sub_name_does_not_crash",
        UxCiTier::Pr,
        Some(UxComponent::Hover),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_hover_harness()?;
            recorder.mark_request_start("hover_sub_declaration");
            let hover_result = harness.hover("calc.pl", 3, 4);
            if hover_result.is_ok() {
                recorder.mark_first_useful_result("hover_sub_declaration");
            }
            recorder.check(
                "hover on sub declaration does not return a JSON-RPC error",
                hover_result.is_ok(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_11_hover_range_contains_cursor_when_present() {
    run_ux_scenario(
        "hover_core",
        SCENARIO_FILE,
        "scenario_11_hover_range_contains_cursor_when_present",
        UxCiTier::Pr,
        Some(UxComponent::Hover),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_hover_harness()?;
            recorder.mark_request_start("hover_call_site_range");
            let hover_result = harness.hover("calc.pl", 8, 14)?;
            recorder.mark_first_useful_result("hover_call_site_range");
            recorder.check(
                "hover range contains cursor when present",
                hover_result
                    .as_ref()
                    .is_none_or(|result| hover_range_contains_cursor_when_present(result, 8, 14)),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
