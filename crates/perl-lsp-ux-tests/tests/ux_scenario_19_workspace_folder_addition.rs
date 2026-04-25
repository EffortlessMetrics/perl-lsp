// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — adding a workspace folder indexes fresh symbols for search.
//!
//! Verifies that adding a folder via `workspace/didChangeWorkspaceFolders`
//! makes symbols from the new root discoverable through `workspace/symbol`
//! without restarting the server.

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::{Duration, Instant};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

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
