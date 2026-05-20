// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — workspace folder addition lifecycle coverage.
//!
//! Combines two BDD coverages for the runtime workspace folder addition flow:
//! - Initial coverage: adding a folder via `workspace/didChangeWorkspaceFolders`
//!   makes symbols from the new root discoverable through `workspace/symbol`
//!   without restarting the server.
//! - Lifecycle coverage: a second workspace folder added at runtime is
//!   reflected in `workspace/symbol` results disambiguated by
//!   `workspaceFolderUri`.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

const SERVICE_A: &str = "\
package ServiceA;\n\
\n\
sub shared_action_4481 {\n\
    return 'a';\n\
}\n\
\n\
1;\n\
";

const SERVICE_B: &str = "\
package ServiceB;\n\
\n\
sub shared_action_4481 {\n\
    return 'b';\n\
}\n\
\n\
1;\n\
";

const CORE_MODULE: &str = "\
package CoreModule;\n\
\n\
sub core_symbol_4197 {\n\
    return 'core';\n\
}\n\
\n\
1;\n\
";

const EXT_MODULE: &str = "\
package ExtModule;\n\
\n\
sub ext_symbol_4197 {\n\
    return 'ext';\n\
}\n\
\n\
1;\n\
";

fn contains_symbol_in_folder(symbols: &[Value], symbol_name: &str, folder_fragment: &str) -> bool {
    symbols.iter().any(|symbol| {
        symbol["name"].as_str() == Some(symbol_name)
            && symbol
                .pointer("/location/uri")
                .and_then(|uri| uri.as_str())
                .is_some_and(|uri| uri.contains(folder_fragment))
    })
}

#[test]
fn scenario_19_added_workspace_folder_symbols_appear() {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_workspace_folder("svc-core", "svc-core")
            .with_file("svc-core/lib/CoreModule.pm", CORE_MODULE)
            .with_file("svc-ext/lib/ExtModule.pm", EXT_MODULE),
    )
    .expect("Failed to create UX harness");

    let before_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_before = Vec::new();
    while Instant::now() < before_deadline {
        symbols_before = harness
            .workspace_symbols("Module")
            .expect("workspace/symbol must not error before folder addition");
        if contains_symbol_in_folder(&symbols_before, "CoreModule", "/svc-core/")
            && !contains_symbol_in_folder(&symbols_before, "ExtModule", "/svc-ext/")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(
        contains_symbol_in_folder(&symbols_before, "CoreModule", "/svc-core/"),
        "Expected CoreModule to be present before folder addition, got: {:?}",
        symbols_before
    );
    assert!(
        !contains_symbol_in_folder(&symbols_before, "ExtModule", "/svc-ext/"),
        "Expected ExtModule to be absent before folder addition, got: {:?}",
        symbols_before
    );

    harness
        .change_workspace_folders(&[("svc-ext", "svc-ext")], &[])
        .expect("workspace folder addition notification must not fail");

    let after_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_after = Vec::new();
    while Instant::now() < after_deadline {
        symbols_after = harness
            .workspace_symbols("Module")
            .expect("workspace/symbol must not error after folder addition");

        if contains_symbol_in_folder(&symbols_after, "CoreModule", "/svc-core/")
            && contains_symbol_in_folder(&symbols_after, "ExtModule", "/svc-ext/")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(
        contains_symbol_in_folder(&symbols_after, "CoreModule", "/svc-core/"),
        "Expected CoreModule to remain after adding svc-ext, got: {:?}",
        symbols_after
    );
    assert!(
        contains_symbol_in_folder(&symbols_after, "ExtModule", "/svc-ext/"),
        "Expected ExtModule symbols to appear after adding svc-ext, got: {:?}",
        symbols_after
    );

    harness.assert_no_crash();
}

fn folder_uris_for(symbols: &[Value], symbol_name: &str) -> BTreeSet<String> {
    symbols
        .iter()
        .filter(|symbol| symbol["name"].as_str() == Some(symbol_name))
        .filter_map(|symbol| symbol.get("workspaceFolderUri").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

#[test]
fn scenario_19_workspace_folder_addition_surfaces_new_symbols() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    // Given: a workspace that starts with only svc-a indexed.
    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_workspace_folder("svc-a", "svc-a")
            .with_file("svc-a/lib/ServiceA.pm", SERVICE_A),
    )?;

    let before = harness.wait_for_workspace_symbols(
        "shared_action_4481",
        Duration::from_secs(10),
        Duration::from_millis(200),
        |symbols| !symbols.is_empty(),
    )?;
    let before_folders = folder_uris_for(&before, "shared_action_4481");
    assert!(
        before_folders.iter().any(|uri| uri.contains("/svc-a/")),
        "Expected svc-a symbol before folder addition, got: {:?}",
        before
    );
    assert!(
        !before_folders.iter().any(|uri| uri.contains("/svc-b/")),
        "Did not expect svc-b symbols before folder addition, got: {:?}",
        before
    );

    // When: a second workspace folder is added and populated.
    harness.workspace.ensure_dir("svc-b")?;
    harness.workspace.write("svc-b/lib/ServiceB.pm", SERVICE_B)?;
    harness.change_workspace_folders(&[("svc-b", "svc-b")], &[])?;

    // Then: workspace/symbol eventually includes both workspace roots.
    let after = harness.wait_for_workspace_symbols(
        "shared_action_4481",
        Duration::from_secs(10),
        Duration::from_millis(200),
        |symbols| {
            let uris = folder_uris_for(symbols, "shared_action_4481");
            uris.iter().any(|uri| uri.contains("/svc-a/"))
                && uris.iter().any(|uri| uri.contains("/svc-b/"))
        },
    )?;

    let after_folders = folder_uris_for(&after, "shared_action_4481");
    assert!(
        after_folders.iter().any(|uri| uri.contains("/svc-a/")),
        "Expected svc-a symbol after addition, got: {:?}",
        after
    );
    assert!(
        after_folders.iter().any(|uri| uri.contains("/svc-b/")),
        "Expected newly added svc-b symbols after addition, got: {:?}",
        after
    );

    harness.assert_no_crash();
    Ok(())
}
