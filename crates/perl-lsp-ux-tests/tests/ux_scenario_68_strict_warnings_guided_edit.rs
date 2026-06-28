//! Scenario 68 - strict/warnings guided-edit receipt.
//!
//! This receipt exercises the deterministic pragma quick-fix journey over a
//! real stdio LSP process. It proves minimal clients receive plain text edits,
//! snippet-capable clients receive `SnippetTextEdit` document changes for the
//! same quick fixes, and deterministic pragma actions are not tagged as
//! generated assistance.

// UX receipt tests intentionally write structured receipts to stderr for --nocapture logs.
#![allow(clippy::print_stderr)]

use anyhow::{Context, Result, anyhow, bail};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, binary_available, missing_binary_skip,
    run_ux_scenario,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_68_strict_warnings_guided_edit.rs";
const FIXTURE_PATH: &str = "lib/My/GuidedEdit.pm";
const FIXTURE_SOURCE: &str = r#"package My::GuidedEdit;

sub hello {
    print "hello\n";
}

1;
"#;
const EXPECTED_APPLIED_SOURCE: &str = r#"package My::GuidedEdit;
use strict;
use warnings;

sub hello {
    print "hello\n";
}

1;
"#;

#[derive(Debug, Serialize)]
struct ActionShapeReport {
    title: String,
    kind: String,
    edit_shape: &'static str,
    has_workspace_edit_metadata: bool,
    has_generated_tag: bool,
}

#[derive(Debug, Serialize)]
struct PlainEditReport {
    diagnostic_count: usize,
    diagnostic_codes: Vec<String>,
    action_count: usize,
    action_titles: Vec<String>,
    action_shapes: Vec<ActionShapeReport>,
    strict_plain_text_edit_present: bool,
    warnings_plain_text_edit_present: bool,
    source_fix_all_present: bool,
    source_fix_all_texts: Vec<String>,
    source_fix_all_applied_matches_expected: bool,
    workspace_edit_metadata_seen: bool,
    generated_tag_seen: bool,
}

#[derive(Debug, Serialize)]
struct SnippetEditReport {
    action_count: usize,
    action_titles: Vec<String>,
    action_shapes: Vec<ActionShapeReport>,
    strict_snippet_value: Option<String>,
    warnings_snippet_value: Option<String>,
    strict_changes_absent: bool,
    warnings_changes_absent: bool,
    workspace_edit_metadata_seen: bool,
    generated_tag_seen: bool,
}

#[derive(Debug)]
struct TextEdit {
    start: usize,
    end: usize,
    new_text: String,
    original_index: usize,
}

fn minimal_harness() -> Result<UxHarness> {
    UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file(FIXTURE_PATH, FIXTURE_SOURCE),
    )
}

fn snippet_harness() -> Result<UxHarness> {
    let mut config = ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
        .with_file(FIXTURE_PATH, FIXTURE_SOURCE);
    config.client_capability_overrides = json!({
        "workspace": {
            "workspaceEdit": {
                "documentChanges": true,
                "snippetEditSupport": true
            }
        }
    });
    UxHarness::new(config)
}

fn request_code_actions(harness: &UxHarness, diagnostics: &[Value]) -> Result<Vec<Value>> {
    let uri = harness.workspace.uri(FIXTURE_PATH);
    let resp = harness.client.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 },
            },
            "context": {
                "diagnostics": diagnostics,
                "triggerKind": 1
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

fn action_titles(actions: &[Value]) -> Vec<String> {
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

fn action_shapes(actions: &[Value]) -> Vec<ActionShapeReport> {
    actions
        .iter()
        .map(|action| ActionShapeReport {
            title: action
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("<missing title>")
                .to_string(),
            kind: action
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("<missing kind>")
                .to_string(),
            edit_shape: edit_shape(action),
            has_workspace_edit_metadata: action.pointer("/edit/metadata").is_some(),
            has_generated_tag: action_has_generated_tag(action),
        })
        .collect()
}

fn edit_shape(action: &Value) -> &'static str {
    if action.pointer("/edit/documentChanges").is_some() {
        if action.to_string().contains("\"snippet\"") {
            return "snippet_document_changes";
        }
        return "document_changes";
    }
    if action.pointer("/edit/changes").is_some() {
        return "plain_changes";
    }
    if action.get("command").is_some() {
        return "command";
    }
    "none"
}

fn action_has_generated_tag(action: &Value) -> bool {
    action
        .get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|tag| tag.as_i64() == Some(1))
}

fn text_edit_new_texts_for_title(actions: &[Value], title: &str, uri: &str) -> Vec<String> {
    actions
        .iter()
        .filter(|action| action.get("title").and_then(Value::as_str) == Some(title))
        .flat_map(|action| text_edit_new_texts(action, uri))
        .collect()
}

fn text_edit_new_texts(action: &Value, uri: &str) -> Vec<String> {
    action
        .pointer("/edit/changes")
        .and_then(Value::as_object)
        .and_then(|changes| changes.get(uri))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|edit| edit.get("newText").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn snippet_value_for_title(actions: &[Value], title: &str, uri: &str) -> Option<String> {
    actions
        .iter()
        .find(|action| action.get("title").and_then(Value::as_str) == Some(title))
        .and_then(|action| snippet_values(action, uri).into_iter().next())
}

fn snippet_values(action: &Value, uri: &str) -> Vec<String> {
    action
        .pointer("/edit/documentChanges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|change| change.pointer("/textDocument/uri").and_then(Value::as_str) == Some(uri))
        .flat_map(|change| change.get("edits").and_then(Value::as_array).into_iter().flatten())
        .filter_map(|edit| edit.pointer("/snippet/value").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn action_changes_absent(actions: &[Value], title: &str) -> bool {
    actions
        .iter()
        .filter(|action| action.get("title").and_then(Value::as_str) == Some(title))
        .all(|action| action.pointer("/edit/changes").is_none())
}

fn source_fix_all_action(actions: &[Value]) -> Option<&Value> {
    actions.iter().find(|action| {
        action.get("title").and_then(Value::as_str) == Some("Fix all auto-fixable issues")
            && action.get("kind").and_then(Value::as_str) == Some("source.fixAll")
    })
}

fn workspace_edit_metadata_seen(actions: &[Value]) -> bool {
    actions.iter().any(|action| action.pointer("/edit/metadata").is_some())
}

fn generated_tag_seen(actions: &[Value]) -> bool {
    actions.iter().any(action_has_generated_tag)
}

fn apply_plain_workspace_edit(source: &str, action: &Value, uri: &str) -> Result<String> {
    let edit_values = action
        .pointer("/edit/changes")
        .and_then(Value::as_object)
        .and_then(|changes| changes.get(uri))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("source.fixAll action missing plain changes for {uri}: {action}"))?;
    let mut edits = Vec::with_capacity(edit_values.len());
    for (original_index, edit) in edit_values.iter().enumerate() {
        let range = edit.get("range").ok_or_else(|| anyhow!("text edit missing range: {edit}"))?;
        let new_text = edit
            .get("newText")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("text edit missing newText: {edit}"))?;
        let (start, end) = range_to_offsets(source, range)?;
        edits.push(TextEdit { start, end, new_text: new_text.to_string(), original_index });
    }

    edits.sort_by(|left, right| {
        right.start.cmp(&left.start).then_with(|| right.original_index.cmp(&left.original_index))
    });

    let mut applied = source.to_string();
    for edit in edits {
        if edit.start > edit.end || edit.end > applied.len() {
            bail!(
                "invalid edit bounds start={} end={} for current len={}",
                edit.start,
                edit.end,
                applied.len()
            );
        }
        applied.replace_range(edit.start..edit.end, &edit.new_text);
    }
    Ok(applied)
}

fn range_to_offsets(source: &str, range: &Value) -> Result<(usize, usize)> {
    let start = range.get("start").ok_or_else(|| anyhow!("range missing start: {range}"))?;
    let end = range.get("end").ok_or_else(|| anyhow!("range missing end: {range}"))?;
    Ok((position_to_offset(source, start)?, position_to_offset(source, end)?))
}

fn position_to_offset(source: &str, position: &Value) -> Result<usize> {
    let line = position
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("position missing line: {position}"))?;
    let character = position
        .get("character")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("position missing character: {position}"))?;
    let line = usize::try_from(line)?;
    let character = usize::try_from(character)?;

    let line_start = if line == 0 {
        0
    } else {
        let mut current_line = 0_usize;
        let mut start = None;
        for (idx, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                current_line += 1;
                if current_line == line {
                    start = Some(idx + 1);
                    break;
                }
            }
        }
        start.ok_or_else(|| anyhow!("line {line} out of bounds for source"))?
    };

    let line_end =
        source[line_start..].find('\n').map(|offset| line_start + offset).unwrap_or(source.len());
    let line_text = source
        .get(line_start..line_end)
        .with_context(|| format!("line {line} is not a valid UTF-8 slice"))?;
    let byte_delta = line_text
        .char_indices()
        .nth(character)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| line_text.len());
    Ok(line_start + byte_delta)
}

fn run_plain_profile() -> Result<PlainEditReport> {
    let harness = minimal_harness()?;
    harness.open_file(FIXTURE_PATH, FIXTURE_SOURCE)?;
    let diagnostics = harness.wait_for_latest_diagnostics(FIXTURE_PATH, Duration::from_secs(5));
    let actions = request_code_actions(&harness, &diagnostics)?;
    let uri = harness.workspace.uri(FIXTURE_PATH);
    let strict_texts = text_edit_new_texts_for_title(&actions, "Add use strict;", &uri);
    let warnings_texts = text_edit_new_texts_for_title(&actions, "Add use warnings;", &uri);
    let source_fix_all = source_fix_all_action(&actions);
    let source_fix_all_texts =
        source_fix_all.map(|action| text_edit_new_texts(action, &uri)).unwrap_or_default();
    let applied_matches_expected = match source_fix_all {
        Some(action) => {
            apply_plain_workspace_edit(FIXTURE_SOURCE, action, &uri)? == EXPECTED_APPLIED_SOURCE
        }
        None => false,
    };

    harness.assert_no_crash();
    Ok(PlainEditReport {
        diagnostic_count: diagnostics.len(),
        diagnostic_codes: diagnostic_codes(&diagnostics),
        action_count: actions.len(),
        action_titles: action_titles(&actions),
        action_shapes: action_shapes(&actions),
        strict_plain_text_edit_present: strict_texts.iter().any(|text| text == "use strict;\n"),
        warnings_plain_text_edit_present: warnings_texts
            .iter()
            .any(|text| text == "use warnings;\n"),
        source_fix_all_present: source_fix_all.is_some(),
        source_fix_all_texts,
        source_fix_all_applied_matches_expected: applied_matches_expected,
        workspace_edit_metadata_seen: workspace_edit_metadata_seen(&actions),
        generated_tag_seen: generated_tag_seen(&actions),
    })
}

fn run_snippet_profile() -> Result<SnippetEditReport> {
    let harness = snippet_harness()?;
    harness.open_file(FIXTURE_PATH, FIXTURE_SOURCE)?;
    let diagnostics = harness.wait_for_latest_diagnostics(FIXTURE_PATH, Duration::from_secs(5));
    let actions = request_code_actions(&harness, &diagnostics)?;
    let uri = harness.workspace.uri(FIXTURE_PATH);
    let strict_snippet_value = snippet_value_for_title(&actions, "Add use strict;", &uri);
    let warnings_snippet_value = snippet_value_for_title(&actions, "Add use warnings;", &uri);

    harness.assert_no_crash();
    Ok(SnippetEditReport {
        action_count: actions.len(),
        action_titles: action_titles(&actions),
        action_shapes: action_shapes(&actions),
        strict_snippet_value,
        warnings_snippet_value,
        strict_changes_absent: action_changes_absent(&actions, "Add use strict;"),
        warnings_changes_absent: action_changes_absent(&actions, "Add use warnings;"),
        workspace_edit_metadata_seen: workspace_edit_metadata_seen(&actions),
        generated_tag_seen: generated_tag_seen(&actions),
    })
}

#[test]
fn scenario_68_strict_warnings_guided_edit_receipt() {
    run_ux_scenario(
        "strict_warnings_guided_edit",
        SCENARIO_FILE,
        "scenario_68_strict_warnings_guided_edit_receipt",
        UxCiTier::Pr,
        Some(UxComponent::CodeActions),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            recorder.mark_request_start("plain_text_edit_code_actions");
            let plain = run_plain_profile()?;
            if plain.strict_plain_text_edit_present && plain.warnings_plain_text_edit_present {
                recorder.mark_first_useful_result("plain_text_edit_code_actions");
            }

            recorder.mark_request_start("snippet_text_edit_code_actions");
            let snippet = run_snippet_profile()?;
            if snippet.strict_snippet_value.as_deref() == Some("use strict;\n")
                && snippet.warnings_snippet_value.as_deref() == Some("use warnings;\n")
            {
                recorder.mark_first_useful_result("snippet_text_edit_code_actions");
            }

            let receipt = json!({
                "schema_version": 1,
                "receipt": "strict_warnings_guided_edit",
                "workspace_fixture": "package missing use strict and use warnings pragmas",
                "claim_boundary": "real-stdio LSP deterministic pragma code-action and source.fixAll aggregation receipt only; no server-originated applyEdit flow, no WorkspaceEdit.metadata, no generated-assistance label",
                "classification": "deterministic_quick_fix",
                "plain_profile": plain,
                "snippet_profile": snippet,
            });
            eprintln!(
                "strict_warnings_guided_edit_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "minimal client received plain strict quick fix",
                plain.strict_plain_text_edit_present,
            )?;
            recorder.check(
                "minimal client received plain warnings quick fix",
                plain.warnings_plain_text_edit_present,
            )?;
            recorder.check(
                "minimal client received source.fixAll for both pragma edits",
                plain.source_fix_all_present
                    && plain.source_fix_all_texts.iter().any(|text| text == "use strict;\n")
                    && plain.source_fix_all_texts.iter().any(|text| text == "use warnings;\n"),
            )?;
            recorder.check(
                "plain source.fixAll applies to the expected final document",
                plain.source_fix_all_applied_matches_expected,
            )?;
            recorder.check(
                "snippet-capable client received strict SnippetTextEdit",
                snippet.strict_snippet_value.as_deref() == Some("use strict;\n")
                    && snippet.strict_changes_absent,
            )?;
            recorder.check(
                "snippet-capable client received warnings SnippetTextEdit",
                snippet.warnings_snippet_value.as_deref() == Some("use warnings;\n")
                    && snippet.warnings_changes_absent,
            )?;
            recorder.check(
                "pragma code actions did not emit WorkspaceEdit.metadata",
                !plain.workspace_edit_metadata_seen && !snippet.workspace_edit_metadata_seen,
            )?;
            recorder.check(
                "deterministic pragma actions were not tagged as generated",
                !plain.generated_tag_seen && !snippet.generated_tag_seen,
            )?;

            Ok(())
        },
    );
}
