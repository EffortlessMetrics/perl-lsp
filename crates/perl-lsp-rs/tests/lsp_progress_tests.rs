//! UX-facing progress tests for LSP 3.17 work done progress.
//!
//! These scenarios validate client-observable behavior (progress lifecycle,
//! report value contracts, and cancel notification resilience) using the
//! end-to-end `LspHarness` protocol path.

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::{LspHarness, TempWorkspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn caps_with_progress() -> serde_json::Value {
    json!({
        "window": {
            "workDoneProgress": true
        }
    })
}

fn make_workspace_with_files(file_count: usize) -> Result<TempWorkspace, String> {
    let ws = TempWorkspace::new()?;
    for i in 0..file_count {
        ws.write(
            &format!("lib/Scenario{i}.pm"),
            &format!("package Scenario{i};\nsub run {{ return {i}; }}\n1;\n"),
        )?;
    }
    Ok(ws)
}

fn progress_timeout() -> Duration {
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(6)
    }
}

#[test]
fn given_progress_enabled_when_workspace_initializes_then_indexing_emits_begin_then_end()
-> TestResult {
    let workspace = make_workspace_with_files(8)?;
    let mut harness = LspHarness::new_raw();

    harness.initialize_with_root(&workspace.root_uri, Some(caps_with_progress()))?;

    let lifecycle = harness
        .wait_for_progress_sequence("workspace-index", &["begin", "end"], progress_timeout())
        .map_err(|e| format!("expected workspace-index begin/end lifecycle: {e}"))?;

    assert_eq!(lifecycle.len(), 2, "expected begin + end lifecycle");
    Ok(())
}

#[test]
fn given_indexing_reports_when_collecting_progress_then_percentages_are_within_bounds() -> TestResult
{
    let workspace = make_workspace_with_files(80)?;
    let mut harness = LspHarness::new_raw();

    harness.initialize_with_root(&workspace.root_uri, Some(caps_with_progress()))?;

    harness
        .wait_for_progress_sequence("workspace-index", &["begin", "end"], progress_timeout())
        .map_err(|e| {
            format!("expected workspace-index lifecycle before validating reports: {e}")
        })?;

    let progress_notifications = harness.drain_notifications(Some("$/progress"), 250);
    let mut report_percentages = Vec::new();

    for notification in &progress_notifications {
        let token = notification.pointer("/params/token").and_then(|v| v.as_str());
        let kind = notification.pointer("/params/value/kind").and_then(|v| v.as_str());

        if token == Some("workspace-index")
            && kind == Some("report")
            && let Some(pct) =
                notification.pointer("/params/value/percentage").and_then(|v| v.as_u64())
        {
            report_percentages.push(pct);
        }
    }

    for pct in &report_percentages {
        assert!(*pct <= 100, "report percentage must be <= 100, got {pct}");
    }

    Ok(())
}

#[test]
fn given_numeric_progress_cancel_when_workspace_indexing_runs_then_server_stays_responsive()
-> TestResult {
    let workspace = make_workspace_with_files(20)?;
    let mut harness = LspHarness::new_raw();

    harness.initialize_with_root(&workspace.root_uri, Some(caps_with_progress()))?;

    harness
        .wait_for_progress_kind("workspace-index", "begin", progress_timeout())
        .map_err(|e| format!("expected workspace-index begin before cancel test: {e}"))?;

    // LSP requires ProgressToken to be string | integer.
    // For this server path, integer tokens are ignored for workspace-index
    // routing and must be handled gracefully.
    harness.notify(
        "window/workDoneProgress/cancel",
        json!({
            "token": 42
        }),
    );

    // Probe responsiveness after cancel notification handling.
    let response = harness
        .request("workspace/symbol", json!({"query": "Scenario"}))
        .map_err(|e| format!("workspace/symbol request failed after cancel: {e}"))?;

    assert!(
        response.is_array(),
        "workspace/symbol should return a JSON array response, got: {response:?}"
    );

    Ok(())
}
