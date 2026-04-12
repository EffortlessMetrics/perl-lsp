//! Fast/slow diagnostic split tests (Issue #4279)
//!
//! Verifies that parse errors are published immediately (fast path) on didChange,
//! while the full diagnostic set follows after the debounce (slow path).
//! The fast path publishes ONLY parse-error-coded diagnostics (PL001).

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

/// Fast path: the first publishDiagnostics notification after didChange must
/// contain only parse-error diagnostics (code "PL001"), not scope-analysis
/// or quality-check diagnostics.
///
/// In production this fires before the 250ms debounce; in tests both fire
/// synchronously so we check that the FIRST notification is parse-errors-only.
#[test]
fn test_fast_path_contains_only_parse_errors() {
    let mut harness = LspHarness::new();
    harness
        .initialize(Some(json!({
            "textDocument": {
                // No pull-diagnostics capability: force push path
            }
        })))
        .expect("initialize should succeed");

    let uri = "file:///fast_slow_test.pl";

    // Open a clean document first
    harness.open(uri, "my $x = 1;\n").expect("open should succeed");

    // Drain any existing notifications (from didOpen)
    let _ = harness.drain_notifications(Some("textDocument/publishDiagnostics"), 400);

    // Now change to a document with a parse error
    harness.change_full(uri, 2, "my $broken = ;\n").expect("change should succeed");

    // Wait for notifications to arrive
    let all_notifications =
        harness.drain_notifications(Some("textDocument/publishDiagnostics"), 600);

    assert!(
        !all_notifications.is_empty(),
        "Expected at least one publishDiagnostics notification after didChange"
    );

    // The FIRST notification must be the fast path: only parse-error diagnostics
    let first = &all_notifications[0];
    let first_diags = first["params"]["diagnostics"].as_array().cloned().unwrap_or_default();

    assert!(
        !first_diags.is_empty(),
        "Fast-path notification must contain parse-error diagnostics, got empty list"
    );

    let parse_error_code = "PL001";
    assert!(
        first_diags
            .iter()
            .all(|d| { d["code"].as_str().map(|c| c == parse_error_code).unwrap_or(false) }),
        "Fast-path publishDiagnostics must ONLY contain parse-error diagnostics \
         (code {}), but found other codes: {:?}",
        parse_error_code,
        first_diags.iter().map(|d| d["code"].as_str()).collect::<Vec<_>>()
    );
}

/// Two-phase delivery: didChange produces two publishDiagnostics notifications.
/// First: parse-errors only (fast). Second: full set (slow, replaces the first).
#[test]
fn test_two_phase_diagnostic_delivery_on_change() {
    let mut harness = LspHarness::new();
    harness
        .initialize(Some(json!({
            "textDocument": {}
        })))
        .expect("initialize should succeed");

    let uri = "file:///two_phase_test.pl";

    harness.open(uri, "my $x = 1;\n").expect("open should succeed");

    // Drain didOpen notifications
    let _ = harness.drain_notifications(Some("textDocument/publishDiagnostics"), 400);

    // Change to a document with a parse error
    harness.change_full(uri, 2, "my $broken = ;\n").expect("change should succeed");

    // Collect all notifications that arrive within a generous window
    let all_notifications =
        harness.drain_notifications(Some("textDocument/publishDiagnostics"), 600);

    assert!(
        all_notifications.len() >= 2,
        "Expected at least 2 publishDiagnostics notifications (fast + slow path), \
         but got {}. First notification: {:?}",
        all_notifications.len(),
        all_notifications.first()
    );

    // First notification: parse errors only
    let first_diags =
        all_notifications[0]["params"]["diagnostics"].as_array().cloned().unwrap_or_default();
    assert!(
        first_diags.iter().all(|d| d["code"].as_str() == Some("PL001")),
        "First (fast-path) notification must contain only parse errors (PL001), \
         got codes: {:?}",
        first_diags.iter().map(|d| d["code"].as_str()).collect::<Vec<_>>()
    );

    // Last notification: full set — must include non-parse-error diagnostics
    let last = all_notifications.last().expect("at least two notifications");
    let last_diags = last["params"]["diagnostics"].as_array().cloned().unwrap_or_default();
    assert!(
        last_diags.iter().any(|d| d["code"].as_str() != Some("PL001")),
        "Last (slow-path) notification must contain the full diagnostic set \
         including non-parse-error codes, but only found: {:?}",
        last_diags.iter().map(|d| d["code"].as_str()).collect::<Vec<_>>()
    );
}
