//! Scenario 15 — workspace symbol search across multiple workspace folders.
//!
//! Verifies that the UX harness can drive `workspace/symbol` in a multi-root
//! workspace and that same-named symbols from different folders remain
//! disambiguatable via `workspaceFolderUri`.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_15_workspace_symbols.rs";

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
    run_ux_scenario(
        "multi_root_workspace_symbols",
        SCENARIO_FILE,
        "scenario_15_workspace_symbol_multi_root_disambiguation",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
                    .env("PERL_LSP_WORKSPACE", "1")
                    .with_workspace_folder("svc-a", "svc-a")
                    .with_workspace_folder("svc-b", "svc-b")
                    .with_file("svc-a/lib/Runner.pm", RUNNER_A)
                    .with_file("svc-b/lib/Runner.pm", RUNNER_B),
            )?;

            recorder.mark_request_start("workspace_symbol_multi_root");

            let deadline = Instant::now() + Duration::from_secs(10);
            let mut latest_symbols = Vec::new();

            while Instant::now() < deadline {
                latest_symbols = harness.workspace_symbols("run")?;

                let run_symbols = matching_run_symbols(&latest_symbols);
                let folder_uris = normalized_folder_uris(&harness, &run_symbols);

                if run_symbols.len() >= 2
                    && folder_uris.len() >= 2
                    && folder_uris.contains("file://$WORKSPACE/svc-a")
                    && folder_uris.contains("file://$WORKSPACE/svc-b")
                {
                    recorder.mark_first_useful_result("workspace_symbol_multi_root");
                    break;
                }

                std::thread::sleep(Duration::from_millis(200));
            }

            let run_symbols = matching_run_symbols(&latest_symbols);
            recorder.check(
                "workspace/symbol returns same-named run entries across both roots",
                run_symbols.len() >= 2,
            )?;

            let folder_uris = normalized_folder_uris(&harness, &run_symbols);

            recorder.check(
                "workspace symbols include distinct workspaceFolderUri values",
                folder_uris.len() >= 2,
            )?;
            recorder.check(
                "one workspace symbol points at svc-a",
                folder_uris.contains("file://$WORKSPACE/svc-a"),
            )?;
            recorder.check(
                "one workspace symbol points at svc-b",
                folder_uris.contains("file://$WORKSPACE/svc-b"),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
