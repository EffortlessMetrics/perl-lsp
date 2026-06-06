//! Scenario 16 — workspace folder removal updates workspace-symbol results.
//!
//! Verifies that removing a folder via `workspace/didChangeWorkspaceFolders`
//! evicts its symbols from `workspace/symbol` results instead of leaving stale
//! cross-folder state behind.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_16_workspace_folder_removal.rs";

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
    run_ux_scenario(
        "workspace_folder_removal_freshness",
        SCENARIO_FILE,
        "scenario_16_removed_workspace_folder_symbols_disappear",
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
                    .with_file("svc-a/lib/ModuleA.pm", MODULE_A)
                    .with_file("svc-b/lib/ModuleB.pm", MODULE_B),
            )?;

            recorder.mark_request_start("workspace_symbols_before_removal");
            let before_deadline = Instant::now() + Duration::from_secs(10);
            let mut symbols_before = Vec::new();
            while Instant::now() < before_deadline {
                symbols_before = harness.workspace_symbols("Module")?;
                if contains_symbol_in_folder(&symbols_before, "ModuleA", "/svc-a/")
                    && contains_symbol_in_folder(&symbols_before, "ModuleB", "/svc-b/")
                {
                    recorder.mark_first_useful_result("workspace_symbols_before_removal");
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            recorder.check(
                "workspace/symbol returned remaining folder symbol before removal",
                contains_symbol_in_folder(&symbols_before, "ModuleA", "/svc-a/"),
            )?;
            recorder.check(
                "workspace/symbol returned removable folder symbol before removal",
                contains_symbol_in_folder(&symbols_before, "ModuleB", "/svc-b/"),
            )?;

            harness.change_workspace_folders(&[], &[("svc-b", "svc-b")])?;

            recorder.mark_request_start("workspace_symbols_after_removal");
            let after_deadline = Instant::now() + Duration::from_secs(10);
            let mut symbols_after = Vec::new();
            while Instant::now() < after_deadline {
                symbols_after = harness.workspace_symbols("Module")?;

                if contains_symbol_in_folder(&symbols_after, "ModuleA", "/svc-a/")
                    && !contains_symbol_in_folder(&symbols_after, "ModuleB", "/svc-b/")
                {
                    recorder.mark_first_useful_result("workspace_symbols_after_removal");
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            recorder.check(
                "workspace/symbol kept remaining folder symbol after removal",
                contains_symbol_in_folder(&symbols_after, "ModuleA", "/svc-a/"),
            )?;
            recorder.check(
                "workspace/symbol removed deleted folder symbol after removal",
                !contains_symbol_in_folder(&symbols_after, "ModuleB", "/svc-b/"),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
