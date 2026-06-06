//! Scenario 23 — Rename workflow UX coverage.
//!
//! Exercises `textDocument/prepareRename` + `textDocument/rename` against a
//! realistic first-session refactor flow.
//!
//! Contract:
//! - `prepareRename` and `rename` MUST NOT return JSON-RPC errors.
//! - simple same-file subroutine rename MUST return a WorkspaceEdit targeting
//!   the opened file.
//! - rename edits MUST cover the declaration and both call sites with the
//!   requested new name.

// Tests print skip reasons when the optional perl-lsp binary is unavailable.
#![allow(clippy::print_stderr)]

use anyhow::{Context, Result};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};
use std::time::Duration;

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

    let result = rename.get("result").context("rename response missing result")?;
    assert!(!result.is_null(), "same-file subroutine rename must return edits: {:?}", rename);

    let edit_count = workspace_edit_count_for_uri(result, &uri)?;
    assert!(
        edit_count >= 3,
        "rename should update declaration + two call-sites; got {edit_count} edits"
    );

    let new_texts = workspace_edit_new_texts_for_uri(result, &uri)?;
    assert_eq!(
        new_texts.len(),
        edit_count,
        "every counted rename edit should include newText; got {new_texts:?}"
    );
    assert!(
        new_texts.iter().all(|new_text| new_text == "welcome"),
        "rename edits should use requested new name; got {new_texts:?}"
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

fn workspace_edit_new_texts_for_uri(workspace_edit: &Value, uri: &str) -> Result<Vec<String>> {
    let mut new_texts = Vec::new();

    if let Some(changes) = workspace_edit.get("changes").and_then(Value::as_object) {
        if let Some(edits) = changes.get(uri).and_then(Value::as_array) {
            new_texts.extend(edits.iter().filter_map(rename_edit_new_text));
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
                new_texts.extend(edits.iter().filter_map(rename_edit_new_text));
            }
        }
    }

    Ok(new_texts)
}

fn rename_edit_new_text(edit: &Value) -> Option<String> {
    edit.get("newText").and_then(Value::as_str).map(ToOwned::to_owned)
}
