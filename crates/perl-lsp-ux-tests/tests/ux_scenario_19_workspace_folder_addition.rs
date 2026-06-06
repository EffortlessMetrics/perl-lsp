//! Scenario 19 — workspace folder addition lifecycle coverage.
//!
//! Combines two BDD coverages for the runtime workspace folder addition flow:
//! - Initial coverage: adding a folder via `workspace/didChangeWorkspaceFolders`
//!   makes symbols from the new root discoverable through `workspace/symbol`
//!   without restarting the server.
//! - Lifecycle coverage: a second workspace folder added at runtime is
//!   reflected in `workspace/symbol` results disambiguated by
//!   `workspaceFolderUri`.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_19_workspace_folder_addition.rs";

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
    run_ux_scenario(
        "workspace_folder_addition_freshness",
        SCENARIO_FILE,
        "scenario_19_added_workspace_folder_symbols_appear",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
                    .env("PERL_LSP_WORKSPACE", "1")
                    .with_workspace_folder("svc-core", "svc-core")
                    .with_file("svc-core/lib/CoreModule.pm", CORE_MODULE)
                    .with_file("svc-ext/lib/ExtModule.pm", EXT_MODULE),
            )?;

            recorder.mark_request_start("workspace_symbols_before_addition");
            let before_deadline = Instant::now() + Duration::from_secs(10);
            let mut symbols_before = Vec::new();
            while Instant::now() < before_deadline {
                symbols_before = harness.workspace_symbols("Module")?;
                if contains_symbol_in_folder(&symbols_before, "CoreModule", "/svc-core/")
                    && !contains_symbol_in_folder(&symbols_before, "ExtModule", "/svc-ext/")
                {
                    recorder.mark_first_useful_result("workspace_symbols_before_addition");
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            recorder.check(
                "workspace/symbol returned initial folder symbol before addition",
                contains_symbol_in_folder(&symbols_before, "CoreModule", "/svc-core/"),
            )?;
            recorder.check(
                "workspace/symbol kept unadded folder absent before addition",
                !contains_symbol_in_folder(&symbols_before, "ExtModule", "/svc-ext/"),
            )?;

            harness.change_workspace_folders(&[("svc-ext", "svc-ext")], &[])?;

            recorder.mark_request_start("workspace_symbols_after_addition");
            let after_deadline = Instant::now() + Duration::from_secs(10);
            let mut symbols_after = Vec::new();
            while Instant::now() < after_deadline {
                symbols_after = harness.workspace_symbols("Module")?;

                if contains_symbol_in_folder(&symbols_after, "CoreModule", "/svc-core/")
                    && contains_symbol_in_folder(&symbols_after, "ExtModule", "/svc-ext/")
                {
                    recorder.mark_first_useful_result("workspace_symbols_after_addition");
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            recorder.check(
                "workspace/symbol kept initial folder symbol after addition",
                contains_symbol_in_folder(&symbols_after, "CoreModule", "/svc-core/"),
            )?;
            recorder.check(
                "workspace/symbol returned added folder symbol after addition",
                contains_symbol_in_folder(&symbols_after, "ExtModule", "/svc-ext/"),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
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
fn scenario_19_workspace_folder_addition_surfaces_new_symbols() {
    run_ux_scenario(
        "workspace_folder_addition_freshness",
        SCENARIO_FILE,
        "scenario_19_workspace_folder_addition_surfaces_new_symbols",
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
                    .with_file("svc-a/lib/ServiceA.pm", SERVICE_A),
            )?;

            recorder.mark_request_start("shared_symbol_before_addition");
            let before = harness.wait_for_workspace_symbols(
                "shared_action_4481",
                Duration::from_secs(10),
                Duration::from_millis(200),
                |symbols| !symbols.is_empty(),
            )?;
            recorder.mark_first_useful_result("shared_symbol_before_addition");

            let before_folders = folder_uris_for(&before, "shared_action_4481");
            recorder.check(
                "workspace/symbol returned initial shared symbol before addition",
                before_folders.iter().any(|uri| uri.contains("/svc-a/")),
            )?;
            recorder.check(
                "workspace/symbol kept second shared symbol absent before addition",
                !before_folders.iter().any(|uri| uri.contains("/svc-b/")),
            )?;

            harness.workspace.ensure_dir("svc-b")?;
            harness.workspace.write("svc-b/lib/ServiceB.pm", SERVICE_B)?;
            harness.change_workspace_folders(&[("svc-b", "svc-b")], &[])?;

            recorder.mark_request_start("shared_symbol_after_addition");
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
            recorder.mark_first_useful_result("shared_symbol_after_addition");

            let after_folders = folder_uris_for(&after, "shared_action_4481");
            recorder.check(
                "workspace/symbol kept initial shared symbol after addition",
                after_folders.iter().any(|uri| uri.contains("/svc-a/")),
            )?;
            recorder.check(
                "workspace/symbol returned added shared symbol after addition",
                after_folders.iter().any(|uri| uri.contains("/svc-b/")),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
