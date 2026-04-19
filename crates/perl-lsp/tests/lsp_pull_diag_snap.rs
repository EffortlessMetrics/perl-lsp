//! Snapshot coverage for pull diagnostics (LSP 3.17).
//!
//! These tests complement assertion-based diagnostics tests by snapshotting the
//! response payload shape and key fields. This makes protocol-visible changes
//! obvious during review.

use insta::assert_yaml_snapshot;
use serde_json::{Value, json};

mod support;
use support::lsp_harness::LspHarness;

fn scrub_result_ids(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.contains_key("resultId") {
                map.insert(
                    "resultId".to_string(),
                    Value::String("<stable-result-id>".into()),
                );
            }
            for nested in map.values_mut() {
                scrub_result_ids(nested);
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_result_ids(item);
            }
        }
        _ => {}
    }
}

#[test]
fn snapshot_document_diagnostic_full_report() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///snapshot-full.pl";
    let content = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
print $y;  # Undefined variable
"#;

    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({})))?;
    harness.open_document(uri, content)?;

    let mut report = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    scrub_result_ids(&mut report);
    assert_yaml_snapshot!("document_diagnostic_full_report", report);
    Ok(())
}

#[test]
fn snapshot_document_diagnostic_unchanged_report() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///snapshot-unchanged.pl";
    let content = r#"#!/usr/bin/perl
print "Hello, World!\\n";
"#;

    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({})))?;
    harness.open_document(uri, content)?;

    let first = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    let previous_result_id = first
        .get("resultId")
        .and_then(Value::as_str)
        .ok_or("missing resultId in first document diagnostic")?;

    let mut unchanged = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": uri },
            "previousResultId": previous_result_id
        }),
    )?;

    scrub_result_ids(&mut unchanged);
    assert_yaml_snapshot!("document_diagnostic_unchanged_report", unchanged);
    Ok(())
}

#[test]
fn snapshot_workspace_diagnostic_reports() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({})))?;

    harness.open_document(
        "file:///workspace-diagnostic-a.pl",
        r#"use strict;
my $x = 1;
print $y;  # Error
"#,
    )?;

    harness.open_document(
        "file:///workspace-diagnostic-b.pl",
        r#"#!/usr/bin/perl
print "OK\\n";
"#,
    )?;

    let mut workspace_result = harness.request("workspace/diagnostic", json!({}))?;

    if let Some(items) = workspace_result
        .get_mut("items")
        .and_then(Value::as_array_mut)
    {
        items.sort_by(|left, right| {
            left.get("uri")
                .and_then(Value::as_str)
                .cmp(&right.get("uri").and_then(Value::as_str))
        });
    }

    scrub_result_ids(&mut workspace_result);
    assert_yaml_snapshot!("workspace_diagnostic_reports", workspace_result);
    Ok(())
}
