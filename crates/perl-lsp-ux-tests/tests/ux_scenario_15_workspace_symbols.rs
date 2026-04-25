// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 15 — workspace symbol search across multiple workspace folders.
//!
//! Verifies that the UX harness can drive `workspace/symbol` in a multi-root
//! workspace and that same-named symbols from different folders remain
//! disambiguatable via `workspaceFolderUri`.

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

const RUNNER_A: &str = "\
package Runner;\n\
\n\
sub run {\n\
    return 'from-a';\n\
}\n\
\n\
1;\n\
";

const RUNNER_B: &str = "\
package Runner;\n\
\n\
sub run {\n\
    return 'from-b';\n\
}\n\
\n\
1;\n\
";

fn matching_run_symbols(symbols: &[Value]) -> Vec<Value> {
    symbols.iter().filter(|symbol| symbol["name"].as_str() == Some("run")).cloned().collect()
}

fn normalized_folder_uris(harness: &UxHarness, symbols: &[Value]) -> BTreeSet<String> {
    symbols
        .iter()
        .filter_map(|symbol| symbol.get("workspaceFolderUri"))
        .map(|uri| harness.normalize_response(&json!({ "workspaceFolderUri": uri })))
        .filter_map(|normalized| {
            normalized.get("workspaceFolderUri").and_then(|uri| uri.as_str()).map(ToOwned::to_owned)
        })
        .collect()
}

#[test]
fn scenario_15_workspace_symbol_multi_root_disambiguation() {
    if !binary_available() {
        eprintln!("SKIP scenario_15: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .env("PERL_LSP_WORKSPACE", "1")
            .with_workspace_folder("svc-a", "svc-a")
            .with_workspace_folder("svc-b", "svc-b")
            .with_file("svc-a/lib/Runner.pm", RUNNER_A)
            .with_file("svc-b/lib/Runner.pm", RUNNER_B),
    )
    .expect("Failed to create UX harness");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut latest_symbols = Vec::new();

    while Instant::now() < deadline {
        latest_symbols = harness
            .workspace_symbols("run")
            .expect("workspace/symbol must not return an error in multi-root mode");

        let run_symbols = matching_run_symbols(&latest_symbols);
        let folder_uris = normalized_folder_uris(&harness, &run_symbols);

        if run_symbols.len() >= 2
            && folder_uris.len() >= 2
            && folder_uris.contains("file://$WORKSPACE/svc-a")
            && folder_uris.contains("file://$WORKSPACE/svc-b")
        {
            break;
        }

        std::thread::sleep(Duration::from_millis(200));
    }

    let run_symbols = matching_run_symbols(&latest_symbols);
    assert!(
        run_symbols.len() >= 2,
        "Expected workspace/symbol to find same-named 'run' entries across both workspace \
         folders, got: {:?}",
        latest_symbols
    );

    let folder_uris = normalized_folder_uris(&harness, &run_symbols);

    assert!(
        folder_uris.len() >= 2,
        "Expected workspace symbols to carry distinct workspaceFolderUri values for multi-root \
         disambiguation, got: {:?}",
        latest_symbols
    );
    assert!(
        folder_uris.contains("file://$WORKSPACE/svc-a"),
        "Expected one workspace symbol to point at svc-a, got: {:?}",
        folder_uris
    );
    assert!(
        folder_uris.contains("file://$WORKSPACE/svc-b"),
        "Expected one workspace symbol to point at svc-b, got: {:?}",
        folder_uris
    );

    harness.assert_no_crash();
}
