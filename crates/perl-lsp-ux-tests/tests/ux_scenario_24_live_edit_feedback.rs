// Test infrastructure needs skip/status messages when the external binary is absent.
#![allow(clippy::print_stderr)]
// Test assertions intentionally panic with UX-specific failure messages.
#![allow(clippy::panic)]

//! Scenario 24 — Live-edit UX feedback loop for diagnostics + definition.
//!
//! BDD contract:
//! - Given a file with an undefined variable, when it is opened, then diagnostics
//!   should surface a strict warning/error for that variable.
//! - Given the declaration was added, when go-to-definition runs on the use-site,
//!   then it should return a definition location in the edited file.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
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
fn given_live_edit_when_variable_is_declared_then_definition_resolves_use_site() -> Result<()> {
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

    harness.change_file_full("live_edit.pl", DECLARED_SOURCE)?;

    let _post_edit_diagnostics =
        harness.wait_for_latest_diagnostics("live_edit.pl", Duration::from_secs(6));

    let definitions =
        harness.definition_with_retry("live_edit.pl", 4, 7, 5, Duration::from_millis(200))?;
    assert!(
        !definitions.is_empty(),
        "expected go-to-definition to resolve declared $name after didChange, got empty result"
    );
    assert!(
        definitions.iter().all(is_lsp_location_shape),
        "definition entries must be Location or LocationLink values: {definitions:?}"
    );
    assert!(
        definitions.iter().any(|entry| entry_uri_ends_with(entry, "live_edit.pl")),
        "expected go-to-definition after didChange to point at live_edit.pl, got: {:?}",
        definitions
    );

    harness.assert_no_crash();
    Ok(())
}

fn is_lsp_location_shape(entry: &Value) -> bool {
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    let is_location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    is_location || is_location_link
}

fn entry_uri_ends_with(entry: &Value, suffix: &str) -> bool {
    entry
        .get("uri")
        .or_else(|| entry.get("targetUri"))
        .and_then(Value::as_str)
        .is_some_and(|uri| uri.replace('\\', "/").ends_with(suffix))
}
