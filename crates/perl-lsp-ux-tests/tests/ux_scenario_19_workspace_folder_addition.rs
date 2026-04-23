// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — adding a workspace folder updates workspace-symbol and definition results.
//!
//! Verifies that adding a new workspace folder via
//! `workspace/didChangeWorkspaceFolders` makes fresh symbols and definition
//! targets discoverable without restarting the server.

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

const APP_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'svc-a/lib';\n\
use lib 'svc-b/lib';\n\
use DynamicAdd;\n\
\n\
my $value = DynamicAdd::joined_after_add_5310();\n\
print \"$value\\n\";\n\
";

const DYNAMIC_MODULE: &str = "\
package DynamicAdd;\n\
\n\
sub joined_after_add_5310 {\n\
    return 5310;\n\
}\n\
\n\
1;\n\
";

fn contains_symbol_in_folder(symbols: &[Value], symbol_name: &str, folder_fragment: &str) -> bool {
    symbols.iter().any(|symbol| {
        symbol["name"].as_str() == Some(symbol_name)
            && symbol
                .pointer("/location/uri")
                .and_then(Value::as_str)
                .is_some_and(|uri| uri.contains(folder_fragment))
    })
}

#[test]
fn scenario_19_added_workspace_folder_symbols_and_definition_appear() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_workspace_folder("svc-a", "svc-a")
            .with_file("main.pl", APP_SOURCE)
            .with_file("svc-b/lib/DynamicAdd.pm", DYNAMIC_MODULE),
    )?;

    harness.open_file("main.pl", APP_SOURCE)?;

    let symbols_before = harness.wait_for_workspace_symbols_until(
        "joined_after_add_5310",
        Duration::from_secs(5),
        |symbols| symbols.is_empty(),
    )?;
    assert!(
        symbols_before.is_empty(),
        "Expected symbol to be absent before adding workspace folder, got: {:?}",
        symbols_before
    );

    let defs_before = harness.definition("main.pl", 6, 29)?;
    assert!(
        defs_before.is_empty(),
        "Expected definition to be unavailable before adding workspace folder, got: {:?}",
        defs_before
    );

    harness.change_workspace_folders(&[("svc-b", "svc-b")], &[])?;

    let symbols_after = harness.wait_for_workspace_symbols_until(
        "joined_after_add_5310",
        Duration::from_secs(10),
        |symbols| contains_symbol_in_folder(symbols, "joined_after_add_5310", "/svc-b/"),
    )?;
    assert!(
        contains_symbol_in_folder(&symbols_after, "joined_after_add_5310", "/svc-b/"),
        "Expected added workspace folder to publish joined_after_add_5310, got: {:?}",
        symbols_after
    );

    let defs_after = harness.wait_for_definition("main.pl", 6, 29, Duration::from_secs(10))?;
    assert!(
        !defs_after.is_empty(),
        "Expected definition to resolve after adding svc-b workspace folder, got: {:?}",
        defs_after
    );

    harness.assert_no_crash();
    Ok(())
}
