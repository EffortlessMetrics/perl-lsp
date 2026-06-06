//! Scenario 18 — Go-to-declaration feature grid coverage.
//!
//! Verifies that `textDocument/declaration` is wired up end-to-end for the LSP
//! server process used in UX regression testing.
//!
//! Contract:
//! - `textDocument/declaration` MUST NOT return a JSON-RPC error.
//! - Same-file subroutine calls MUST resolve to the source declaration.
//! - Each result MUST include URI/range shape (`targetUri` +
//!   `targetRange` for links, or `uri` + `range` for locations).

// Binary skip messages are visible only in integration-test output.
#![allow(clippy::print_stderr)]

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;

const INC_CALL_LINE: u32 = 10;
const INC_CALL_CHARACTER: u32 = 13;
const INC_DECLARATION_LINE: u64 = 5;

const DECLARATION_FIXTURE: &str = r#"use strict;
use warnings;

my $value = 41;

sub inc {
    my ($n) = @_;
    return $n + 1;
}

my $result = inc($value);
print "$result\n";
"#;

#[test]
fn scenario_18_declaration_request_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("declaration.pl", DECLARATION_FIXTURE))?;

    harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;
    let result = harness.declaration("declaration.pl", INC_CALL_LINE, INC_CALL_CHARACTER);

    assert!(
        result.is_ok(),
        "textDocument/declaration must not return a JSON-RPC error — feature grid regression: {:?}",
        result
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_18_declaration_result_points_to_sub_inc() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("declaration.pl", DECLARATION_FIXTURE))?;

    harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;
    let declarations = harness.declaration("declaration.pl", INC_CALL_LINE, INC_CALL_CHARACTER)?;

    assert!(
        !declarations.is_empty(),
        "goto-declaration on `inc($value)` must return the `sub inc` declaration"
    );

    for entry in &declarations {
        assert!(
            is_location_shape(entry),
            "declaration result must be LocationLink or Location, got: {:?}",
            entry
        );
    }

    let points_to_inc = declarations.iter().any(|entry| {
        entry_uri(entry).is_some_and(|uri| uri.ends_with("declaration.pl"))
            && entry_target_start_line(entry) == Some(INC_DECLARATION_LINE)
    });
    assert!(
        points_to_inc,
        "goto-declaration on `inc($value)` must point to `sub inc` on line \
         {INC_DECLARATION_LINE}, got: {declarations:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

fn is_location_shape(entry: &Value) -> bool {
    let is_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    is_link || is_location
}

fn entry_uri(entry: &Value) -> Option<&str> {
    entry.get("targetUri").or_else(|| entry.get("uri")).and_then(Value::as_str)
}

fn entry_target_start_line(entry: &Value) -> Option<u64> {
    entry
        .pointer("/targetSelectionRange/start/line")
        .or_else(|| entry.pointer("/targetRange/start/line"))
        .or_else(|| entry.pointer("/range/start/line"))
        .and_then(Value::as_u64)
}
