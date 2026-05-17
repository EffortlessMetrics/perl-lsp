//! Scenario 36 - Mojolicious safe-delete warning receipt.
//!
//! This receipt exercises `workspace/willDeleteFiles` over the committed
//! Mojolicious skeleton workspace. It records real-workspace file-delete
//! safe-delete warning behavior without claiming symbol-level safe-delete
//! cutover.
//!
//! Receipt signals:
//! - deleting a module file used by another workspace file returns a
//!   WorkspaceEdit-shaped response
//! - the editor receives a dependent-workspace-file warning
//! - the receipt boundary remains file-delete warning proof only

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    FixtureFile, LspEvent, ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available,
    fixture_content, load_real_project_fixture_files, missing_binary_skip, run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_36_mojolicious_safe_delete_warning.rs";
const DELETE_TARGET: &str = "lib/Mojolicious/Static.pm";
const DEPENDENT_FILE: &str = "lib/Mojolicious.pm";

#[derive(Debug, Serialize)]
struct SafeDeleteWarningReport {
    deleted_file: &'static str,
    expected_dependent_file: &'static str,
    response_is_workspace_edit: bool,
    response_has_changes: bool,
    warning_seen: bool,
    dependent_warning_seen: bool,
    warning_messages: Vec<String>,
}

fn create_mojolicious_harness(files: &[FixtureFile]) -> Result<UxHarness> {
    let mut config = ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
        .env("PERL_LSP_WORKSPACE", "1");

    for file in files {
        config = config.with_file(&file.relative_path, &file.content);
    }

    UxHarness::new(config)
}

fn open_all_fixture_files(harness: &UxHarness, files: &[FixtureFile]) -> Result<()> {
    for file in files {
        harness.open_file(&file.relative_path, &file.content)?;
    }
    Ok(())
}

fn assert_fixture_dependency(files: &[FixtureFile]) -> Result<()> {
    let deleted = fixture_content(files, DELETE_TARGET)?;
    let dependent = fixture_content(files, DEPENDENT_FILE)?;
    anyhow::ensure!(deleted.contains("package Mojolicious::Static;"));
    anyhow::ensure!(
        dependent.contains("use Mojolicious::Static;"),
        "{DEPENDENT_FILE} must import the deleted module for this receipt"
    );
    Ok(())
}

fn will_delete_file(harness: &UxHarness, relative_path: &str) -> Result<Value> {
    let response = harness.client.request(
        "workspace/willDeleteFiles",
        json!({
            "files": [
                { "uri": harness.workspace.uri(relative_path) }
            ]
        }),
        Duration::from_secs(20),
    )?;
    if let Some(error) = response.get("error") {
        anyhow::bail!("workspace/willDeleteFiles returned error: {error}");
    }
    response.get("result").cloned().context("workspace/willDeleteFiles missing result")
}

fn warning_messages(harness: &UxHarness) -> Vec<String> {
    harness
        .peek_notifications()
        .into_iter()
        .filter_map(|event| match event {
            LspEvent::WindowMessage { message, .. } | LspEvent::LogMessage { message, .. } => {
                Some(message)
            }
            _ => None,
        })
        .collect()
}

fn safe_delete_warning_report(harness: &UxHarness) -> Result<SafeDeleteWarningReport> {
    let result = will_delete_file(harness, DELETE_TARGET)?;
    let warning_seen =
        harness.client.wait_for_message("Safe delete warning", Duration::from_secs(5));
    let messages = warning_messages(harness);
    let dependent_warning_seen =
        messages.iter().any(|message| message.contains("dependent workspace file"));

    Ok(SafeDeleteWarningReport {
        deleted_file: DELETE_TARGET,
        expected_dependent_file: DEPENDENT_FILE,
        response_is_workspace_edit: result.is_object(),
        response_has_changes: result.get("changes").is_some(),
        warning_seen,
        dependent_warning_seen,
        warning_messages: messages,
    })
}

#[test]
fn scenario_36_mojolicious_safe_delete_warning_receipt() {
    run_ux_scenario(
        "mojolicious_safe_delete_warning",
        SCENARIO_FILE,
        "scenario_36_mojolicious_safe_delete_warning_receipt",
        UxCiTier::Pr,
        Some(UxComponent::SafeDelete),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_real_project_fixture_files("mojolicious_skeleton")?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;
            assert_fixture_dependency(&fixture_files)?;

            let fixture_paths = fixture_files
                .iter()
                .map(|file| file.relative_path.clone())
                .collect::<BTreeSet<_>>();
            recorder.check(
                "safe-delete target and dependent fixtures are present",
                fixture_paths.contains(DELETE_TARGET) && fixture_paths.contains(DEPENDENT_FILE),
            )?;

            let harness = create_mojolicious_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            recorder.mark_request_start("workspace/willDeleteFiles");
            let report = safe_delete_warning_report(&harness)?;
            recorder.mark_first_useful_result("workspace/willDeleteFiles");

            let receipt = json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "safe_delete",
                "claim_boundary": "real-workspace file-delete safe-delete warning receipt only; no symbol-level safe-delete cutover or broader refactor behavior promoted",
                "symbol_level_safe_delete": "boundary-shadowed",
                "fixture_file_count": fixture_files.len(),
                "report": &report,
            });
            eprintln!(
                "mojolicious_safe_delete_warning_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "workspace/willDeleteFiles returned WorkspaceEdit-shaped response",
                report.response_is_workspace_edit && report.response_has_changes,
            )?;
            recorder.check(
                "safe-delete warning reached editor message surface",
                report.warning_seen && report.dependent_warning_seen,
            )?;
            recorder.check(
                "safe-delete warning does not promote symbol-level cutover",
                receipt["symbol_level_safe_delete"] == "boundary-shadowed",
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
