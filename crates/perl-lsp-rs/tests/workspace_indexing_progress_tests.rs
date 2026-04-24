//! Workspace indexing progress reporting tests.
//!
//! Covers issues #2317 and #2356: the LSP server must send `$/progress`
//! notifications (begin -> report* -> end) during workspace indexing when the
//! client advertises `window.workDoneProgress = true`.

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;
use support::test_workspace::TempWorkspace;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Capability set that enables work-done progress.
fn caps_with_progress() -> serde_json::Value {
    json!({
        "window": {
            "workDoneProgress": true
        }
    })
}

/// Capability set without work-done progress.
fn caps_without_progress() -> serde_json::Value {
    json!({
        "window": {
            "workDoneProgress": false
        }
    })
}

// ---------------------------------------------------------------------------
// Helper: create a workspace with N Perl files.
// ---------------------------------------------------------------------------
fn make_workspace_with_files(n: usize) -> Result<TempWorkspace, String> {
    let ws = TempWorkspace::new()?;
    for i in 0..n {
        ws.write(
            &format!("lib/Module{i}.pm"),
            &format!("package Module{i};\nsub new {{ return bless {{}} }}\n1;\n"),
        )?;
    }
    Ok(ws)
}

/// Adaptive timeout: longer in CI or constrained environments.
fn progress_timeout() -> Duration {
    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    if is_ci { Duration::from_secs(10) } else { Duration::from_secs(5) }
}

// ---------------------------------------------------------------------------
// Test 1: progress begin is sent when client supports workDoneProgress.
// ---------------------------------------------------------------------------
#[test]
fn progress_begin_sent_on_indexing_start() -> TestResult {
    let ws = make_workspace_with_files(5)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_with_progress()))?;

    harness
        .wait_for_progress_kind("workspace-index", "begin", progress_timeout())
        .map_err(|e| format!("expected $/progress begin: {e}"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: progress end is sent when indexing completes.
// ---------------------------------------------------------------------------
#[test]
fn progress_end_sent_when_indexing_completes() -> TestResult {
    let ws = make_workspace_with_files(5)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_with_progress()))?;

    // Wait for both begin and end.
    harness
        .wait_for_progress_kind("workspace-index", "begin", progress_timeout())
        .map_err(|e| format!("expected $/progress begin: {e}"))?;
    harness
        .wait_for_progress_kind("workspace-index", "end", progress_timeout())
        .map_err(|e| format!("expected $/progress end: {e}"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: no $/progress sent when client does not support workDoneProgress.
// ---------------------------------------------------------------------------
#[test]
fn no_progress_sent_when_capability_absent() -> TestResult {
    let ws = make_workspace_with_files(5)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_without_progress()))?;

    // Give the background thread time to finish indexing.
    std::thread::sleep(Duration::from_millis(500));

    let progress_notifications = harness.drain_notifications(Some("$/progress"), 200);

    // Filter to workspace-index token specifically.
    let ws_progress: Vec<_> = progress_notifications
        .iter()
        .filter(|n| n.pointer("/params/token").and_then(|v| v.as_str()) == Some("workspace-index"))
        .collect();

    assert!(
        ws_progress.is_empty(),
        "expected no $/progress for workspace-index when capability is absent; \
         got: {ws_progress:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 4: progress sequence is well-formed (begin before end).
// ---------------------------------------------------------------------------
#[test]
fn progress_sequence_is_ordered() -> TestResult {
    let ws = make_workspace_with_files(5)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_with_progress()))?;

    let timeout = progress_timeout();

    let begin = harness.wait_for_progress_kind("workspace-index", "begin", timeout);
    let end = harness.wait_for_progress_kind("workspace-index", "end", timeout);

    // Both begin and end must arrive.
    assert!(begin.is_ok(), "expected begin: {begin:?}");
    assert!(end.is_ok(), "expected end: {end:?}");

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 5: percentage in report notifications is in 0..=100.
// ---------------------------------------------------------------------------
#[test]
fn progress_report_percentage_is_valid() -> TestResult {
    // Use a larger workspace to increase the chance of report notifications.
    let ws = make_workspace_with_files(60)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_with_progress()))?;

    let timeout = Duration::from_secs(8);

    // Wait for begin first.
    let _ = harness.wait_for_progress_kind("workspace-index", "begin", timeout);

    // Collect any report notifications that arrived within the timeout.
    let notifications = harness.drain_notifications(Some("$/progress"), timeout.as_millis() as u64);

    let report_notifications: Vec<_> = notifications
        .iter()
        .filter(|n| {
            n.pointer("/params/token").and_then(|v| v.as_str()) == Some("workspace-index")
                && n.pointer("/params/value/kind").and_then(|v| v.as_str()) == Some("report")
        })
        .collect();

    for report in &report_notifications {
        if let Some(pct) = report.pointer("/params/value/percentage").and_then(|v| v.as_u64()) {
            assert!(pct <= 100, "progress percentage must be <= 100, got {pct} in {report}");
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 6: window/workDoneProgress/create is sent as a server-to-client
//         request when the client supports workDoneProgress.
// ---------------------------------------------------------------------------
#[test]
fn work_done_progress_create_sent_to_client() -> TestResult {
    let ws = make_workspace_with_files(5)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_with_progress()))?;

    // Wait for begin (which implies create was sent first).
    harness
        .wait_for_progress_kind("workspace-index", "begin", progress_timeout())
        .map_err(|e| format!("expected $/progress begin (after create): {e}"))?;

    // Now drain server-initiated requests.
    let server_requests = harness.drain_server_requests(1_000);

    let create_found = server_requests.iter().any(|req| {
        req.get("method").and_then(|m| m.as_str()) == Some("window/workDoneProgress/create")
            && req.pointer("/params/token").and_then(|v| v.as_str()) == Some("workspace-index")
    });

    assert!(
        create_found,
        "expected window/workDoneProgress/create for token 'workspace-index'; \
         got server_requests: {server_requests:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 7: racing workspace folder change during initial indexing does not
//         produce duplicate $/progress begin notifications (issue #2641).
// ---------------------------------------------------------------------------
#[test]
fn no_duplicate_progress_begin_on_concurrent_reindex() -> TestResult {
    let ws = make_workspace_with_files(5)?;
    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, Some(caps_with_progress()))?;

    // Immediately send workspace folder change to race with initial indexing.
    harness.notify(
        "workspace/didChangeWorkspaceFolders",
        json!({ "event": { "added": [], "removed": [] } }),
    );

    // Wait for indexing to complete.
    let _ = harness.wait_for_progress_kind("workspace-index", "end", progress_timeout());

    // Collect all $/progress notifications.
    let notifications = harness.drain_notifications(Some("$/progress"), 3_000);
    let begins: Vec<_> = notifications
        .iter()
        .filter(|n| {
            n.pointer("/params/token").and_then(|v| v.as_str()) == Some("workspace-index")
                && n.pointer("/params/value/kind").and_then(|v| v.as_str()) == Some("begin")
        })
        .collect();

    // Must have at most one begin for the workspace-index token.
    assert!(
        begins.len() <= 1,
        "expected at most 1 progress begin, got {}: {begins:?}",
        begins.len()
    );
    Ok(())
}
