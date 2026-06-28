//! Scenario 24 — Live-edit UX feedback loop for diagnostics + definition.
//!
//! BDD contract:
//! - Given a file with an undefined variable, when it is opened, then diagnostics
//!   should surface a strict warning/error for that variable.
//! - Given the declaration was added, when go-to-definition runs on the use-site,
//!   then it should stay responsive and return either locations or an empty
//!   result (degraded mode).

use anyhow::{Context, Result};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

const UNDECLARED_SOURCE: &str = r#"use strict;
use warnings;

print $name;
"#;

const DECLARED_SOURCE: &str = r#"use strict;
use warnings;

my $name = 'world';
print $name;
"#;

fn has_global_symbol_diagnostic(diags: &[serde_json::Value], symbol: &str) -> bool {
    diags.iter().any(|diag| {
        let message = diag.get("message").and_then(serde_json::Value::as_str).unwrap_or_default();
        let code = diag.get("code").and_then(serde_json::Value::as_str).unwrap_or_default();
        message.contains(symbol) || (code.contains("Global symbol") && message.contains(symbol))
    })
}

#[test]
fn given_undeclared_variable_when_opened_then_strict_diagnostic_is_published() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_24: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("live_edit.pl", UNDECLARED_SOURCE))?;

    harness.open_file("live_edit.pl", UNDECLARED_SOURCE)?;

    let diagnostics = harness.wait_for_diagnostics("live_edit.pl", Duration::from_secs(6));
    assert!(
        has_global_symbol_diagnostic(&diagnostics, "$name"),
        "expected strict diagnostics for undeclared $name, got: {:?}",
        diagnostics
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn given_live_edit_when_variable_is_declared_then_navigation_remains_responsive() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_24: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("live_edit.pl", UNDECLARED_SOURCE))?;

    harness.open_file("live_edit.pl", UNDECLARED_SOURCE)?;

    let before = harness.wait_for_diagnostics("live_edit.pl", Duration::from_secs(6));
    assert!(
        has_global_symbol_diagnostic(&before, "$name"),
        "precondition failed: expected undeclared symbol diagnostic before edit, got: {:?}",
        before
    );

    let diagnostics_seen_before_edit = harness.diagnostics_event_count("live_edit.pl");
    harness.change_file_full("live_edit.pl", DECLARED_SOURCE)?;

    let post_edit_diagnostics = harness
        .wait_for_diagnostics_after_count(
            "live_edit.pl",
            diagnostics_seen_before_edit,
            Duration::from_secs(6),
        )
        .context("expected diagnostics after declaring $name")?;
    assert!(
        !has_global_symbol_diagnostic(&post_edit_diagnostics, "$name"),
        "expected declared $name diagnostic to clear after edit, got: {:?}",
        post_edit_diagnostics
    );

    let definitions = harness.definition("live_edit.pl", 4, 7);
    assert!(
        definitions.is_ok(),
        "expected go-to-definition to stay responsive after didChange, got: {:?}",
        definitions
    );

    harness.assert_no_crash();
    Ok(())
}
