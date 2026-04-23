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
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{json, Value};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

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
fn scenario_23_prepare_rename_and_rename_do_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_23: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("rename_flow.pl", RENAME_FIXTURE))?;
    harness.open_file("rename_flow.pl", RENAME_FIXTURE)?;

    let uri = harness.workspace.uri("rename_flow.pl");

    let prepare = harness.client.request(
        "textDocument/prepareRename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 7, "character": 12 }
        }),
        REQUEST_TIMEOUT,
    )?;
    assert!(
        prepare.get("error").is_none(),
        "prepareRename must not return JSON-RPC error: {:?}",
        prepare
    );

    let rename = harness.client.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 7, "character": 12 },
            "newName": "welcome"
        }),
        REQUEST_TIMEOUT,
    )?;
    assert!(rename.get("error").is_none(), "rename must not return JSON-RPC error: {:?}", rename);

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_23_rename_workspace_edit_targets_file_and_multiple_occurrences() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_23: perl-lsp binary not found");
        return Ok(());
    }
    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("rename_flow.pl", RENAME_FIXTURE))?;
    harness.open_file("rename_flow.pl", RENAME_FIXTURE)?;

    let uri = harness.workspace.uri("rename_flow.pl");

    let rename = harness.client.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 7, "character": 12 },
            "newName": "welcome"
        }),
        REQUEST_TIMEOUT,
    )?;

    assert!(rename.get("error").is_none(), "rename returned JSON-RPC error: {:?}", rename);

    let Some(result) = rename.get("result") else {
        return Ok(());
    };
    if result.is_null() {
        return Ok(());
    }

    let edit_count = workspace_edit_count_for_uri(result, &uri)?;
    assert!(
        edit_count >= 2,
        "rename should update at least declaration + one call-site when edits are returned; got {edit_count} edits"
    );

    harness.assert_no_crash();
    Ok(())
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
