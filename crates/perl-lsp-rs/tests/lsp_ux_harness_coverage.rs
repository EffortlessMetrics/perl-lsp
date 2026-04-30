//! UX-focused coverage for the LSP test harness.
//!
//! These tests validate that the harness convenience methods map to realistic
//! editor workflows (initialize/open/edit/save/close + workspace symbol waits).

#[path = "support/mod.rs"]
mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;

#[test]
fn harness_supports_edit_save_diagnostics_workflow() -> Result<(), String> {
    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;

    let uri = "file:///workspace/workflow.pl";
    harness.open_document(uri, "my $value = ;\n")?;

    let first =
        harness.wait_for_notification("textDocument/publishDiagnostics", Duration::from_secs(2))?;
    let first_uri = first
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "publishDiagnostics missing params.uri".to_string())?;
    if first_uri != uri {
        return Err(format!("Expected diagnostics for {uri}, got {first_uri}"));
    }

    let first_count =
        first.get("diagnostics").and_then(|v| v.as_array()).map_or(0, std::vec::Vec::len);
    if first_count == 0 {
        return Err("Expected at least one diagnostic for broken document".to_string());
    }

    harness.change_full(uri, 2, "my $value = 42;\n")?;
    harness.did_save(uri)?;

    let mut saw_followup_diagnostics = false;
    for _ in 0..8 {
        let batch = harness.drain_notifications(Some("textDocument/publishDiagnostics"), 250);
        for notif in batch {
            let notif_uri = notif.pointer("/params/uri").and_then(|v| v.as_str());
            if notif_uri == Some(uri) {
                saw_followup_diagnostics = true;
                break;
            }
        }
        if saw_followup_diagnostics {
            break;
        }
    }

    if !saw_followup_diagnostics {
        return Err("Expected follow-up diagnostics notification after change + save".to_string());
    }

    harness.close(uri)?;
    Ok(())
}

#[test]
fn harness_with_workspace_and_wait_for_symbol_matches_file_uri() -> Result<(), String> {
    let (mut harness, workspace) = LspHarness::with_workspace(&[
        (
            "lib/MyApp/Greeting.pm",
            "package MyApp::Greeting;\nsub greet {\n    return 'hi';\n}\n1;\n",
        ),
        ("main.pl", "use lib 'lib';\nuse MyApp::Greeting;\nprint MyApp::Greeting::greet();\n"),
    ])?;

    let target_uri = workspace.uri("lib/MyApp/Greeting.pm");
    harness.wait_for_symbol("greet", Some(&target_uri), Duration::from_secs(4))?;

    let symbols = harness.request(
        "workspace/symbol",
        json!({
            "query": "greet"
        }),
    )?;

    let found = symbols.as_array().is_some_and(|arr| {
        arr.iter().any(|s| {
            s.pointer("/location/uri").and_then(|u| u.as_str()) == Some(target_uri.as_str())
        })
    });

    if !found {
        return Err(format!("Expected workspace/symbol to include {target_uri}"));
    }

    Ok(())
}

#[test]
fn harness_timed_request_reports_duration_and_result() -> Result<(), String> {
    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;

    let (result, elapsed) = harness.timed_request("workspace/symbol", json!({ "query": "" }))?;

    if !result.is_array() {
        return Err("workspace/symbol should return an array result".to_string());
    }
    if elapsed == Duration::ZERO {
        return Err("timed_request should report a non-zero elapsed duration".to_string());
    }

    Ok(())
}
