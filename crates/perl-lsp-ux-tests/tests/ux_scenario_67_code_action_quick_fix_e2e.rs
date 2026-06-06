//! Scenario 67 - code-action quick-fix e2e receipt.
//!
//! Exercises the editor-facing diagnostic-to-code-action workflow over a real
//! stdio LSP process. The scenario opens a strict Perl file with an undeclared
//! variable, waits for a published diagnostic, requests `textDocument/codeAction`
//! with the editor diagnostic context, and verifies that declaration quick fixes
//! come back with concrete workspace edits.

// UX receipt tests intentionally write structured receipts to stderr for --nocapture logs.
#![allow(clippy::print_stderr)]

use anyhow::{Context, Result, anyhow};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_67_code_action_quick_fix_e2e.rs";
const FIXTURE_PATH: &str = "script/code-action.pl";
const FIXTURE_SOURCE: &str = r#"use strict;
use warnings;

print $missing;
"#;

#[derive(Debug, Serialize)]
struct CodeActionReceipt {
    schema_version: u8,
    receipt: &'static str,
    workspace_fixture: &'static str,
    claim_boundary: &'static str,
    diagnostic_count: usize,
    diagnostic_codes: Vec<String>,
    action_count: usize,
    action_titles: Vec<String>,
    has_my_declare: bool,
    has_our_declare: bool,
    my_edit_texts: Vec<String>,
    our_edit_texts: Vec<String>,
}

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file(FIXTURE_PATH, FIXTURE_SOURCE),
    )
}

fn position_after(source: &str, needle: &str) -> Result<(u32, u32)> {
    let byte_offset =
        source.find(needle).with_context(|| format!("missing `{needle}`"))? + needle.len();
    position_from_byte_offset(source, byte_offset)
}

fn position_from_byte_offset(source: &str, byte_offset: usize) -> Result<(u32, u32)> {
    let prefix = source
        .get(..byte_offset)
        .with_context(|| format!("byte offset {byte_offset} is not a UTF-8 boundary"))?;
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().map(str::chars).map(Iterator::count).unwrap_or(0);
    Ok((u32::try_from(line)?, u32::try_from(character)?))
}

fn code_action_titles(actions: &[Value]) -> Vec<String> {
    actions
        .iter()
        .filter_map(|action| action.get("title").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn diagnostic_codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.get("code"))
        .filter_map(|code| code.as_str().map(str::to_string).or_else(|| Some(code.to_string())))
        .collect()
}

fn action_edit_texts(actions: &[Value], title: &str, uri: &str) -> Vec<String> {
    actions
        .iter()
        .filter(|action| action.get("title").and_then(Value::as_str) == Some(title))
        .filter_map(|action| action.pointer("/edit/changes"))
        .filter_map(Value::as_object)
        .filter_map(|changes| changes.get(uri))
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|edit| edit.get("newText").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn request_code_actions(
    harness: &UxHarness,
    relative_path: &str,
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
    diagnostics: &[Value],
    only: &[&str],
) -> Result<Vec<Value>> {
    let uri = harness.workspace.uri(relative_path);
    let resp = harness.client.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": start_character },
                "end": { "line": end_line, "character": end_character },
            },
            "context": {
                "diagnostics": diagnostics,
                "only": only,
            },
        }),
        Duration::from_secs(20),
    )?;
    if resp.get("error").is_some() {
        return Err(anyhow!("codeAction returned error: {}", resp["error"]));
    }
    match resp["result"].as_array() {
        Some(actions) => Ok(actions.clone()),
        None if resp["result"].is_null() => Ok(Vec::new()),
        None => Ok(vec![resp["result"].clone()]),
    }
}

#[test]
fn scenario_67_code_action_quick_fix_e2e_receipt() {
    run_ux_scenario(
        "code_action_quick_fix_e2e",
        SCENARIO_FILE,
        "scenario_67_code_action_quick_fix_e2e_receipt",
        UxCiTier::Pr,
        Some(UxComponent::CodeActions),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file(FIXTURE_PATH, FIXTURE_SOURCE)?;

            recorder.mark_request_start("published_diagnostics");
            let diagnostics =
                harness.wait_for_latest_diagnostics(FIXTURE_PATH, Duration::from_secs(5));
            if !diagnostics.is_empty() {
                recorder.mark_first_useful_result("published_diagnostics");
            }

            let (line, end_character) = position_after(FIXTURE_SOURCE, "$missing")?;
            let start_character = end_character.saturating_sub(u32::try_from("$missing".len())?);

            recorder.mark_request_start("code_action_quickfix");
            let actions = request_code_actions(
                &harness,
                FIXTURE_PATH,
                line,
                start_character,
                line,
                end_character,
                &diagnostics,
                &["quickfix"],
            )?;
            if !actions.is_empty() {
                recorder.mark_first_useful_result("code_action_quickfix");
            }

            let uri = harness.workspace.uri(FIXTURE_PATH);
            let action_titles = code_action_titles(&actions);
            let my_title = "Declare '$missing' with 'my'";
            let our_title = "Declare '$missing' with 'our'";
            let has_my_declare = action_titles.iter().any(|title| title == my_title);
            let has_our_declare = action_titles.iter().any(|title| title == our_title);
            let my_edit_texts = action_edit_texts(&actions, my_title, &uri);
            let our_edit_texts = action_edit_texts(&actions, our_title, &uri);

            let receipt = CodeActionReceipt {
                schema_version: 1,
                receipt: "code_action_quick_fix_e2e",
                workspace_fixture: "strict Perl script with one undeclared lexical variable",
                claim_boundary: "real-stdio LSP codeAction quick-fix receipt only; no provider promotion, release action, or behavior change",
                diagnostic_count: diagnostics.len(),
                diagnostic_codes: diagnostic_codes(&diagnostics),
                action_count: actions.len(),
                action_titles,
                has_my_declare,
                has_our_declare,
                my_edit_texts,
                our_edit_texts,
            };
            eprintln!(
                "code_action_quick_fix_e2e_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("undeclared-variable diagnostic was published", !diagnostics.is_empty())?;
            recorder.check("declare-with-my quick fix was returned", receipt.has_my_declare)?;
            recorder.check("declare-with-our quick fix was returned", receipt.has_our_declare)?;
            recorder.check(
                "declare-with-my quick fix contains a concrete workspace edit",
                receipt.my_edit_texts.iter().any(|text| text.contains("my $missing;")),
            )?;
            recorder.check(
                "declare-with-our quick fix contains a concrete workspace edit",
                receipt.our_edit_texts.iter().any(|text| text.contains("our $missing;")),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
