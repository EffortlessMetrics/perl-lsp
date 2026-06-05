//! Scenario 23 — Rename workflow UX coverage.
//!
//! Exercises `textDocument/prepareRename` + `textDocument/rename` against a
//! realistic first-session refactor flow.
//!
//! Contract:
//! - `prepareRename` and `rename` MUST NOT return JSON-RPC errors.
//! - `rename` MAY return null in degraded mode, but if edits are returned they
//!   MUST target the opened file and update multiple occurrences.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_23_rename_workflow.rs";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

const RENAME_FIXTURE: &str = r#"use strict;
use warnings;

sub greet {
    return "hello";
}

my $value = greet();
print greet();
"#;

#[test]
fn scenario_23_prepare_rename_and_rename_do_not_error() {
    run_ux_scenario(
        "rename_workflow_core",
        SCENARIO_FILE,
        "scenario_23_prepare_rename_and_rename_do_not_error",
        UxCiTier::Pr,
        Some(UxComponent::Rename),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig::default().with_file("rename_flow.pl", RENAME_FIXTURE),
            )?;
            harness.open_file("rename_flow.pl", RENAME_FIXTURE)?;

            let uri = harness.workspace.uri("rename_flow.pl");

            recorder.mark_request_start("prepare_rename");
            let prepare = harness.client.request(
                "textDocument/prepareRename",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": 7, "character": 12 }
                }),
                REQUEST_TIMEOUT,
            )?;
            let prepare_clean = prepare.get("error").is_none();
            if prepare_clean {
                recorder.mark_first_useful_result("prepare_rename");
            }
            recorder.check("prepareRename does not return a JSON-RPC error", prepare_clean)?;

            recorder.mark_request_start("rename");
            let rename = harness.client.request(
                "textDocument/rename",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": 7, "character": 12 },
                    "newName": "welcome"
                }),
                REQUEST_TIMEOUT,
            )?;
            let rename_clean = rename.get("error").is_none();
            if rename_clean {
                recorder.mark_first_useful_result("rename");
            }
            recorder.check("rename does not return a JSON-RPC error", rename_clean)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_23_rename_workspace_edit_targets_file_and_multiple_occurrences() {
    run_ux_scenario(
        "rename_workflow_core",
        SCENARIO_FILE,
        "scenario_23_rename_workspace_edit_targets_file_and_multiple_occurrences",
        UxCiTier::Pr,
        Some(UxComponent::Rename),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig::default().with_file("rename_flow.pl", RENAME_FIXTURE),
            )?;
            harness.open_file("rename_flow.pl", RENAME_FIXTURE)?;

            let uri = harness.workspace.uri("rename_flow.pl");

            recorder.mark_request_start("rename_workspace_edit");
            let rename = harness.client.request(
                "textDocument/rename",
                json!({
                    "textDocument": { "uri": uri },
                    "position": { "line": 7, "character": 12 },
                    "newName": "welcome"
                }),
                REQUEST_TIMEOUT,
            )?;

            let rename_clean = rename.get("error").is_none();
            let rename_has_result = rename.get("result").is_some();
            if rename_clean && rename_has_result {
                recorder.mark_first_useful_result("rename_workspace_edit");
            }
            recorder.check("rename returns no JSON-RPC error", rename_clean)?;
            recorder
                .check("rename returns a result field when it does not error", rename_has_result)?;

            let Some(result) = rename.get("result") else {
                harness.assert_no_crash();
                return Ok(());
            };
            if result.is_null() {
                recorder.check("rename returned clean null in degraded mode", true)?;
                harness.assert_no_crash();
                return Ok(());
            }

            let edit_count = workspace_edit_count_for_uri(result, &uri)?;
            recorder.check(
                "rename workspace edit targets opened file with multiple occurrences",
                edit_count >= 2,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

fn workspace_edit_count_for_uri(workspace_edit: &Value, uri: &str) -> Result<usize> {
    if let Some(changes) = workspace_edit.get("changes").and_then(Value::as_object) {
        if let Some(edits) = changes.get(uri).and_then(Value::as_array) {
            return Ok(edits.len());
        }
    }

    if let Some(document_changes) = workspace_edit.get("documentChanges").and_then(Value::as_array)
    {
        for change in document_changes {
            let text_document = change
                .get("textDocument")
                .and_then(Value::as_object)
                .context("rename documentChanges entry missing textDocument")?;
            let entry_uri = text_document
                .get("uri")
                .and_then(Value::as_str)
                .context("rename documentChanges.textDocument.uri missing")?;
            if entry_uri == uri {
                let edits = change
                    .get("edits")
                    .and_then(Value::as_array)
                    .context("rename documentChanges entry missing edits")?;
                return Ok(edits.len());
            }
        }
    }

    Ok(0)
}
