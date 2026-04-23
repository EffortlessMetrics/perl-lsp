//! Scenario 19 — incremental edit churn refreshes symbol and definition results.
//!
//! Verifies that a full-text `textDocument/didChange` update evicts stale symbol
//! names and stale go-to-definition targets from UX-visible requests.

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::{Duration, Instant};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

const MODULE_V1: &str = r#"package Churn;

sub churn_value_old_19 {
    return 19;
}

1;
"#;

const MODULE_V2: &str = r#"package Churn;

sub churn_value_new_19 {
    return 29;
}

1;
"#;

const SCRIPT_SOURCE: &str = r#"use strict;
use warnings;
use lib 'lib';
use Churn;

my $value = Churn::churn_value_old_19();
print "$value\n";
"#;

fn has_symbol(symbols: &[Value], symbol_name: &str) -> bool {
    symbols.iter().any(|entry| entry.get("name").and_then(Value::as_str) == Some(symbol_name))
}

fn definition_points_to(hits: &[Value], file_suffix: &str) -> bool {
    hits.iter().any(|location| {
        location.get("uri").and_then(Value::as_str).is_some_and(|uri| uri.ends_with(file_suffix))
    })
}

#[test]
fn scenario_19_didchange_evicts_stale_symbol_and_definition() -> Result<()> {
    if !binary_available() {
        eprintln!("Skipping scenario_19: perl-lsp binary not available");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig::default()
            .with_file("lib/Churn.pm", MODULE_V1)
            .with_file("main.pl", SCRIPT_SOURCE),
    )?;

    harness.open_file("lib/Churn.pm", MODULE_V1)?;
    harness.open_file("main.pl", SCRIPT_SOURCE)?;

    let deadline = Instant::now() + Duration::from_secs(6);
    let mut saw_prechange_symbol = false;
    let mut saw_prechange_definition = false;

    while Instant::now() < deadline {
        let symbols = harness.workspace_symbols("churn_value_old_19")?;
        let defs = harness.definition("main.pl", 5, 24)?;

        if has_symbol(&symbols, "churn_value_old_19") {
            saw_prechange_symbol = true;
        }
        if definition_points_to(&defs, "/lib/Churn.pm") {
            saw_prechange_definition = true;
        }

        if saw_prechange_symbol && saw_prechange_definition {
            break;
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    assert!(saw_prechange_symbol, "Expected old symbol before edit");
    assert!(saw_prechange_definition, "Expected definition target before edit");

    harness.change_file_full("lib/Churn.pm", MODULE_V2)?;

    let deadline = Instant::now() + Duration::from_secs(8);
    let mut stale_symbol_gone = false;
    let mut fresh_symbol_present = false;
    let mut stale_definition_gone = false;

    while Instant::now() < deadline {
        let old_symbols = harness.workspace_symbols("churn_value_old_19")?;
        let new_symbols = harness.workspace_symbols("churn_value_new_19")?;
        let defs = harness.definition("main.pl", 5, 24)?;

        stale_symbol_gone = !has_symbol(&old_symbols, "churn_value_old_19");
        fresh_symbol_present = has_symbol(&new_symbols, "churn_value_new_19");
        stale_definition_gone = !definition_points_to(&defs, "/lib/Churn.pm");

        if stale_symbol_gone && fresh_symbol_present && stale_definition_gone {
            break;
        }

        std::thread::sleep(Duration::from_millis(120));
    }

    assert!(stale_symbol_gone, "Expected stale symbol to disappear after didChange");
    assert!(fresh_symbol_present, "Expected new symbol to appear after didChange");
    assert!(stale_definition_gone, "Expected stale definition target to disappear after didChange");

    harness.assert_no_crash();
    Ok(())
}
