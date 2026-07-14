// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 16 — workspace folder removal updates workspace-symbol results.
//!
//! Verifies that removing a folder via `workspace/didChangeWorkspaceFolders`
//! evicts its symbols from `workspace/symbol` results instead of leaving stale
//! cross-folder state behind.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::{Duration, Instant};

const MODULE_A: &str = "\
package ModuleA;\n\
\n\
sub alpha {\n\
    return 'a';\n\
}\n\
\n\
1;\n\
";

const MODULE_B: &str = "\
package ModuleB;\n\
\n\
sub beta {\n\
    return 'b';\n\
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
fn scenario_16_removed_workspace_folder_symbols_disappear() {
    if !binary_available() {
        eprintln!("SKIP scenario_16: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_workspace_folder("svc-a", "svc-a")
            .with_workspace_folder("svc-b", "svc-b")
            .with_file("svc-a/lib/ModuleA.pm", MODULE_A)
            .with_file("svc-b/lib/ModuleB.pm", MODULE_B),
    )
    .expect("Failed to create UX harness");

    assert!(
        harness.wait_for_index_ready(Duration::from_secs(20)),
        "Expected initial workspace index to become ready before querying multi-root symbols"
    );

    let before_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_before_a = Vec::new();
    while Instant::now() < before_deadline {
        symbols_before_a = harness
            .workspace_symbols("ModuleA")
            .expect("workspace/symbol must not error before folder removal");
        if contains_symbol_in_folder(&symbols_before_a, "ModuleA", "/svc-a/") {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(
        contains_symbol_in_folder(&symbols_before_a, "ModuleA", "/svc-a/"),
        "Expected ModuleA to be present before folder removal, got: {:?}",
        symbols_before_a
    );

    let before_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_before_b = Vec::new();
    while Instant::now() < before_deadline {
        symbols_before_b = harness
            .workspace_symbols("ModuleB")
            .expect("workspace/symbol must not error before folder removal");
        if contains_symbol_in_folder(&symbols_before_b, "ModuleB", "/svc-b/") {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    assert!(
        contains_symbol_in_folder(&symbols_before_b, "ModuleB", "/svc-b/"),
        "Expected ModuleB to be present before folder removal, got: {:?}",
        symbols_before_b
    );

    harness
        .change_workspace_folders(&[], &[("svc-b", "svc-b")])
        .expect("workspace folder removal notification must not fail");

    let after_deadline = Instant::now() + Duration::from_secs(10);
    let mut symbols_after_b = Vec::new();
    while Instant::now() < after_deadline {
        symbols_after_b = harness
            .workspace_symbols("ModuleB")
            .expect("workspace/symbol must not error after folder removal");

        if !contains_symbol_in_folder(&symbols_after_b, "ModuleB", "/svc-b/") {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    let symbols_after_a = harness
        .workspace_symbols("ModuleA")
        .expect("workspace/symbol must not error after folder removal");
    assert!(
        contains_symbol_in_folder(&symbols_after_a, "ModuleA", "/svc-a/"),
        "Expected ModuleA to remain after removing svc-b, got: {:?}",
        symbols_after_a
    );
    assert!(
        !contains_symbol_in_folder(&symbols_after_b, "ModuleB", "/svc-b/"),
        "Expected ModuleB symbols to disappear after removing svc-b, got: {:?}",
        symbols_after_b
    );

    harness.assert_no_crash();
}
