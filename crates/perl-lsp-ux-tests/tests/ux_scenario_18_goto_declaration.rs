//! Scenario 18 — Go-to-declaration feature grid coverage.
//!
//! Verifies that `textDocument/declaration` is wired up end-to-end for the LSP
//! server process used in UX regression testing.
//!
//! Contract:
//! - `textDocument/declaration` MUST NOT return a JSON-RPC error.
//! - A declaration result MAY be empty (degraded mode acceptable) but must not crash.
//! - When non-empty, each result MUST include URI/range shape (`targetUri` +
//!   `targetRange` for links, or `uri` + `range` for locations).

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};

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
    let result = harness.declaration("declaration.pl", 9, 13);

    assert!(
        result.is_ok(),
        "textDocument/declaration must not return a JSON-RPC error — feature grid regression: {:?}",
        result
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_18_declaration_result_is_location_or_empty() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("declaration.pl", DECLARATION_FIXTURE))?;

    harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;
    let declarations = harness.declaration("declaration.pl", 9, 13)?;

    for entry in declarations {
        let is_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
        let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
        assert!(
            is_link || is_location,
            "declaration result must be LocationLink or Location, got: {:?}",
            entry
        );
    }

    harness.assert_no_crash();
    Ok(())
}
