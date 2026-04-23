//! Scenario 18 — Go-to-declaration UX behavior coverage.
//!
//! BDD acceptance focus:
//! - Given a local symbol declaration, when a user invokes goto-declaration on
//!   the symbol usage, then the result should resolve to the declaration line.
//! - Given an unknown position, when goto-declaration runs, then it should
//!   degrade to an empty result without JSON-RPC errors.
//! - For all non-empty results, payload entries must be valid Location or
//!   LocationLink objects.

use anyhow::Result;
use perl_lsp_ux_tests::{
    ScenarioConfig, UxHarness, all_locations_or_links, has_target_in_file, has_target_start_line,
};
use std::time::Duration;

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

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn new_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("declaration.pl", DECLARATION_FIXTURE),
    )
}

#[test]
fn scenario_18_given_sub_call_when_declaration_requested_then_points_to_sub_definition()
-> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18 sub declaration: perl-lsp binary not found");
        return Ok(());
    }

    let harness = new_harness()?;

    // Given
    harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;

    // When: line 10 contains `inc($value)`.
    let declarations = harness.declaration("declaration.pl", 10, 13)?;

    // Then
    assert!(
        !declarations.is_empty(),
        "Expected goto-declaration on sub call to resolve to a declaration target"
    );
    assert!(
        all_locations_or_links(&declarations),
        "goto-declaration must return Location/LocationLink entries, got: {:?}",
        declarations
    );
    assert!(
        has_target_in_file(&declarations, "declaration.pl"),
        "Expected declaration target to remain in declaration.pl, got: {:?}",
        declarations
    );
    assert!(
        has_target_start_line(&declarations, 5),
        "Expected sub declaration to resolve to line 5 (0-indexed), got: {:?}",
        declarations
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_18_given_variable_usage_when_declaration_requested_then_points_to_variable_binding()
-> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18 variable declaration: perl-lsp binary not found");
        return Ok(());
    }

    let harness = new_harness()?;

    // Given
    harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;

    // When: line 10 contains `inc($value)` and `$value` starts at char 17.
    let declarations = harness.declaration("declaration.pl", 10, 18)?;

    // Then
    assert!(
        !declarations.is_empty(),
        "Expected goto-declaration on `$value` usage to resolve to its binding"
    );
    assert!(
        all_locations_or_links(&declarations),
        "goto-declaration must return Location/LocationLink entries, got: {:?}",
        declarations
    );
    assert!(
        has_target_start_line(&declarations, 3),
        "Expected `$value` declaration target to start at line 3 (0-indexed), got: {:?}",
        declarations
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_18_given_non_symbol_position_when_declaration_requested_then_returns_empty_or_wellformed()
-> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18 empty declaration: perl-lsp binary not found");
        return Ok(());
    }

    let harness = new_harness()?;

    // Given
    harness.open_file("declaration.pl", DECLARATION_FIXTURE)?;

    // When: line 2 is blank.
    let declarations = harness.declaration("declaration.pl", 2, 0)?;

    // Then: either graceful empty or fully valid entries.
    if !declarations.is_empty() {
        assert!(
            all_locations_or_links(&declarations),
            "Non-empty declaration response must contain only Location/LocationLink entries: {:?}",
            declarations
        );
    }

    harness.assert_no_crash();
    Ok(())
}
