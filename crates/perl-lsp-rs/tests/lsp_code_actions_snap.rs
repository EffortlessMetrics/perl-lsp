//! Snapshot coverage for code action payloads.
//!
//! These tests focus on high-signal protocol output:
//! - mixed quickfix + refactor action sets
//! - `context.only` filtering behavior

use insta::assert_yaml_snapshot;
use serde_json::{Value, json};

mod support;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn normalize_actions(response: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let actions = response.as_array().ok_or("code actions response was not an array")?;

    let mut normalized: Vec<Value> = actions
        .iter()
        .map(|action| {
            let kind = action.get("kind").cloned().unwrap_or(Value::Null);
            let title = action.get("title").cloned().unwrap_or(Value::Null);
            let has_edit = Value::Bool(action.get("edit").is_some());
            let has_command = Value::Bool(action.get("command").is_some());
            json!({
                "kind": kind,
                "title": title,
                "has_edit": has_edit,
                "has_command": has_command,
            })
        })
        .collect();

    normalized.sort_by(|left, right| {
        let left_kind = left.get("kind").and_then(Value::as_str).unwrap_or("");
        let right_kind = right.get("kind").and_then(Value::as_str).unwrap_or("");
        left_kind.cmp(right_kind).then_with(|| {
            let left_title = left.get("title").and_then(Value::as_str).unwrap_or("");
            let right_title = right.get("title").and_then(Value::as_str).unwrap_or("");
            left_title.cmp(right_title)
        })
    });

    Ok(Value::Array(normalized))
}

fn request_actions(harness: &mut LspHarness, uri: &str, only: Option<Vec<&str>>) -> TestResult {
    let mut context = json!({ "diagnostics": [] });
    if let Some(only_kinds) = only {
        context["only"] = json!(only_kinds);
    }

    let response = harness.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": 3, "character": 0 },
                "end": { "line": 3, "character": 30 }
            },
            "context": context
        }),
    )?;

    let normalized = normalize_actions(response)?;
    let snapshot_name = if context.get("only").is_some() {
        "code_actions_refactor_only"
    } else {
        "code_actions_unfiltered"
    };

    assert_yaml_snapshot!(snapshot_name, normalized);
    Ok(())
}

#[test]
fn snapshot_code_actions_for_open_call_unfiltered_and_filtered() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///code-actions-snapshot.pl";
    harness.open_document(
        uri,
        r#"use strict;
use warnings;

open(my $fh, '<', 'data.txt');
"#,
    )?;

    request_actions(&mut harness, uri, None)?;
    request_actions(&mut harness, uri, Some(vec!["refactor"]))?;

    Ok(())
}
