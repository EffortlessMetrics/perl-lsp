//! Scenario 17 — deleting a watched file evicts stale symbols and definition targets.
//!
//! Verifies that a real `workspace/didChangeWatchedFiles` Deleted event removes
//! stale search results and cross-file definitions from the UX surface.

use anyhow::Context;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_17_deleted_file_churn.rs";

const MODULE_SOURCE: &str = "\
package ModuleGone;\n\
\n\
sub gone_value_4068 {\n\
    return 42;\n\
}\n\
\n\
1;\n\
";

const SCRIPT_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use ModuleGone;\n\
\n\
my $value = ModuleGone::gone_value_4068();\n\
print \"$value\\n\";\n\
";

#[test]
fn scenario_17_deleted_module_evicted_from_symbols_and_definition() {
    run_ux_scenario(
        "deleted_file_churn_freshness",
        SCENARIO_FILE,
        "scenario_17_deleted_module_evicted_from_symbols_and_definition",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
                    .env("PERL_LSP_WORKSPACE", "1")
                    .with_file("main.pl", SCRIPT_SOURCE)
                    .with_file("lib/ModuleGone.pm", MODULE_SOURCE),
            )?;

            harness.open_file("main.pl", SCRIPT_SOURCE)?;

            let cursor = harness.position_cursor("main.pl", 5, 25);
            let before_deadline = Instant::now() + Duration::from_secs(10);
            let mut symbols_before = Vec::new();
            let mut defs_before = Vec::new();
            let mut symbols_before_ready = false;
            let mut definition_before_ready = false;

            recorder.mark_request_start("workspace_symbols_before_delete");
            recorder.mark_request_start("definition_before_delete");
            while Instant::now() < before_deadline {
                symbols_before = harness.workspace_symbols("gone_value_4068")?;
                if !symbols_before.is_empty() && !symbols_before_ready {
                    recorder.mark_first_useful_result("workspace_symbols_before_delete");
                    symbols_before_ready = true;
                }

                defs_before = harness.definition_at(&cursor)?;
                if !defs_before.is_empty() && !definition_before_ready {
                    recorder.mark_first_useful_result("definition_before_delete");
                    definition_before_ready = true;
                }

                if symbols_before_ready && definition_before_ready {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            recorder.check(
                "deleted module symbol is searchable before delete",
                !symbols_before.is_empty(),
            )?;
            recorder.check(
                "deleted module definition resolves before delete",
                !defs_before.is_empty(),
            )?;
            let first_def_before =
                defs_before.first().context("definition result present before delete")?;
            let normalized_def_before = harness.normalize_response(first_def_before);
            recorder.check(
                "definition target points at module before delete",
                normalized_def_before.get("uri").and_then(Value::as_str)
                    == Some("file://$WORKSPACE/lib/ModuleGone.pm"),
            )?;

            harness.workspace.delete("lib/ModuleGone.pm")?;
            harness.notify_watched_files(&[("lib/ModuleGone.pm", 3)])?;

            let after_deadline = Instant::now() + Duration::from_secs(10);
            let mut symbols_after = Vec::new();
            let mut defs_after = Vec::new();
            let mut symbols_after_fresh = false;
            let mut definition_after_fresh = false;

            recorder.mark_request_start("workspace_symbols_after_delete");
            recorder.mark_request_start("definition_after_delete");
            while Instant::now() < after_deadline {
                symbols_after = harness.workspace_symbols("gone_value_4068")?;
                if symbols_after.is_empty() && !symbols_after_fresh {
                    recorder.mark_first_useful_result("workspace_symbols_after_delete");
                    symbols_after_fresh = true;
                }

                defs_after = harness.definition_at(&cursor)?;
                if defs_after.is_empty() && !definition_after_fresh {
                    recorder.mark_first_useful_result("definition_after_delete");
                    definition_after_fresh = true;
                }

                if symbols_after_fresh && definition_after_fresh {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }

            recorder.check(
                "deleted symbol disappears from workspace/symbol",
                symbols_after.is_empty(),
            )?;
            recorder.check(
                "deleted module definition target disappears after delete",
                defs_after.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
