//! Tests that the server emits a `window/logMessage` notification when the
//! workspace index enters Building state (issue #4190).
//!
//! When the client opens a workspace folder but the index has not yet
//! accumulated any symbols, the server stays in Building state.  The user
//! sees degraded features (go-to-definition, workspace/symbol) with no
//! explanation.  This test asserts that the server sends a
//! `window/logMessage` (Info level) so the user can see the message in
//! the Output panel.

mod support;

use std::thread;
use std::time::{Duration, Instant};
use support::lsp_harness::LspHarness;
use support::test_workspace::TempWorkspace;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Adaptive timeout: longer in CI / coverage environments.
fn log_msg_timeout() -> Duration {
    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    let is_coverage = std::env::var("LLVM_PROFILE_FILE").is_ok()
        || std::env::var("CARGO_LLVM_COV").is_ok()
        || std::env::var("CARGO_LLVM_COV_TARGET_DIR").is_ok();
    if is_ci || is_coverage { Duration::from_secs(10) } else { Duration::from_secs(5) }
}

/// Create a workspace with a few Perl files so the background indexer takes
/// non-trivial time.  Also ensures workspace_folders is non-empty.
fn make_workspace() -> Result<TempWorkspace, String> {
    let ws = TempWorkspace::new()?;
    for i in 0..5 {
        ws.write(
            &format!("lib/Mod{i}.pm"),
            &format!("package Mod{i};\nsub new {{ return bless {{}} }}\n1;\n"),
        )?;
    }
    Ok(ws)
}

/// Wait up to `timeout` for a `window/logMessage` whose text contains `needle`.
///
/// The test harness's `drain_notifications` stops as soon as any notification
/// arrives in the pre-parsed buffer, which can miss messages that are written
/// in a later outbound batch.  This helper polls in short slices to collect
/// all notifications progressively.
fn wait_for_log_message(
    harness: &mut LspHarness,
    needle: &str,
    timeout: Duration,
) -> Result<serde_json::Value, String> {
    let start = Instant::now();
    let slice_ms = 100u64;

    loop {
        // Drain whatever arrived in the last slice.
        let batch = harness.drain_notifications(None, slice_ms);

        for msg in batch {
            if msg.get("method").and_then(|m| m.as_str()) == Some("window/logMessage") {
                let text = msg.pointer("/params/message").and_then(|v| v.as_str()).unwrap_or("");
                if text.to_lowercase().contains(&needle.to_lowercase()) {
                    return Ok(msg);
                }
            }
        }

        if start.elapsed() >= timeout {
            return Err(format!(
                "window/logMessage containing '{}' not received within {:?}",
                needle, timeout
            ));
        }

        // Short sleep to avoid spinning when no messages are arriving.
        thread::sleep(Duration::from_millis(20));
    }
}

// ---------------------------------------------------------------------------
// Test 1: server emits window/logMessage when index enters Building state.
// ---------------------------------------------------------------------------
#[test]
fn building_state_emits_log_message() -> TestResult {
    // Create a workspace directory so `workspace_folders` is non-empty.
    let ws = make_workspace().map_err(|e| e.to_string())?;

    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, None)?;

    let building_msg = wait_for_log_message(&mut harness, "index", log_msg_timeout())
        .map_err(|e| format!("Missing indexing log message: {e}"))?;

    // Verify message type is Info (3) — non-intrusive.
    let msg_type = building_msg.pointer("/params/type").and_then(|v| v.as_i64()).unwrap_or(0);

    assert_eq!(
        msg_type, 3,
        "expected Info (type=3) log message, got type={msg_type} in msg={building_msg}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: message is sent only once per transition (no spam).
// ---------------------------------------------------------------------------
#[test]
fn building_state_log_message_sent_once() -> TestResult {
    let ws = make_workspace().map_err(|e| e.to_string())?;

    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, None)?;

    // Wait to collect all logMessage notifications for a fixed window.
    let timeout = log_msg_timeout();
    let start = Instant::now();
    let mut indexing_msgs: Vec<serde_json::Value> = Vec::new();

    while start.elapsed() < timeout {
        let batch = harness.drain_notifications(None, 200);
        for msg in batch {
            if msg.get("method").and_then(|m| m.as_str()) == Some("window/logMessage")
                && msg
                    .pointer("/params/message")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase().contains("index"))
                    .unwrap_or(false)
            {
                indexing_msgs.push(msg);
            }
        }
        // Early exit once we've seen at least one and waited a bit for duplicates.
        if !indexing_msgs.is_empty() && start.elapsed() > Duration::from_millis(500) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert!(
        indexing_msgs.len() <= 1,
        "expected at most 1 indexing log message per Building transition, \
         got {}: {indexing_msgs:?}",
        indexing_msgs.len()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: message mentions that features will be available when done.
// ---------------------------------------------------------------------------
#[test]
fn building_log_message_is_actionable() -> TestResult {
    let ws = make_workspace().map_err(|e| e.to_string())?;

    let mut harness = LspHarness::new_raw();
    harness.initialize_with_root(&ws.root_uri, None)?;

    // Best-effort: only validates content if message arrives.
    // Test 1 is the authoritative presence check.
    if let Ok(msg) = wait_for_log_message(&mut harness, "index", log_msg_timeout()) {
        let text = msg.pointer("/params/message").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            text.to_lowercase().contains("available") || text.to_lowercase().contains("complet"),
            "expected message to say features will become available, got: {text}"
        );
    }

    Ok(())
}
