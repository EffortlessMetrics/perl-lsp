//! Tests for opt-in parse error telemetry wiring (issue #2137)
//!
//! Verifies that when `perl.telemetry.enabled` is true, parse errors in
//! documents produce `telemetry/event` notifications with privacy-safe
//! metadata, and that no telemetry is sent when disabled (the default).
//!
//! Privacy invariants enforced here:
//! - No file path components in telemetry payload (only extension)
//! - No source code content in telemetry payload
//! - No user identity information

mod support;

use std::time::Duration;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ============================================================
// Helpers
// ============================================================

/// Enable telemetry via didChangeConfiguration and sync with barrier.
fn enable_telemetry(harness: &mut LspHarness) {
    harness.notify(
        "workspace/didChangeConfiguration",
        serde_json::json!({
            "settings": {
                "perl": {
                    "telemetry": {
                        "enabled": true
                    }
                }
            }
        }),
    );
    // Use barrier to ensure config change is processed before proceeding
    harness.barrier();
}

// ============================================================
// Test 1: telemetry/event sent on parse error when enabled
// ============================================================

#[test]
fn test_parse_telemetry_sent_when_enabled() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    enable_telemetry(&mut harness);

    // Open a document with invalid Perl that will produce a parse error
    harness.open("file:///test_telem.pl", "my $x = ;")?;

    // Wait for telemetry/event notification — generous timeout for CI/parallel runs
    let notif = harness.wait_for_notification("telemetry/event", Duration::from_millis(800));

    assert!(
        notif.is_ok(),
        "Expected telemetry/event notification for parse error, but got: {:?}",
        notif.err()
    );

    let payload = notif?;

    // Verify required fields are present
    assert_eq!(
        payload["type"].as_str(),
        Some("parseError"),
        "telemetry event type must be 'parseError'"
    );

    let error_types = payload["errorTypes"].as_array();
    assert!(
        error_types.is_some() && !error_types.unwrap_or(&vec![]).is_empty(),
        "telemetry payload must contain non-empty errorTypes"
    );

    let degradation_tier = payload["degradationTier"].as_str();
    assert!(degradation_tier.is_some(), "telemetry payload must contain degradationTier");

    Ok(())
}

// ============================================================
// Test 2: No telemetry when disabled (default)
// ============================================================

#[test]
fn test_parse_telemetry_not_sent_when_disabled() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Do NOT enable telemetry — it must be off by default

    harness.open("file:///test_disabled.pl", "my $x = ;")?;

    // Drain notifications over a short window — should find no telemetry/event
    let notifications = harness.drain_notifications(Some("telemetry/event"), 150);

    assert!(
        notifications.is_empty(),
        "Expected no telemetry/event when telemetry is disabled (default), but got: {:?}",
        notifications
    );

    Ok(())
}

// ============================================================
// Test 3: No telemetry for a clean parse
// ============================================================

#[test]
fn test_parse_telemetry_not_sent_for_clean_parse() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    enable_telemetry(&mut harness);

    // Open a document with valid Perl — no parse errors
    harness.open("file:///clean_telem.pl", "use strict;\nuse warnings;\nmy $x = 1;\n")?;

    // Drain notifications — should find no telemetry/event for a clean parse
    let notifications = harness.drain_notifications(Some("telemetry/event"), 200);

    assert!(
        notifications.is_empty(),
        "Expected no telemetry/event for a clean parse, but got: {:?}",
        notifications
    );

    Ok(())
}

// ============================================================
// Test 4: Rate limiting — second parse error within cooldown suppressed
// ============================================================

#[test]
fn test_parse_telemetry_rate_limited() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    enable_telemetry(&mut harness);

    let uri = "file:///rate_limit_telem.pl";

    // First open — should emit telemetry; wait up to 500ms for it
    harness.open(uri, "my $x = ;")?;
    let first_notif = harness.wait_for_notification("telemetry/event", Duration::from_millis(800));
    assert!(
        first_notif.is_ok(),
        "Expected telemetry/event on first open, got: {:?}",
        first_notif.err()
    );

    // Immediately re-trigger diagnostics for same URI via didChange — still within cooldown
    harness.change_full(uri, 2, "my $y = ;")?;

    // Should NOT fire a second telemetry event (rate-limited)
    let second_notifications = harness.drain_notifications(Some("telemetry/event"), 200);
    assert!(
        second_notifications.is_empty(),
        "Expected no second telemetry/event within cooldown window, but got: {:?}",
        second_notifications
    );

    Ok(())
}

// ============================================================
// Test 5: PII exclusion — file path components must not appear
// ============================================================

#[test]
fn test_parse_telemetry_no_pii_in_payload() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    enable_telemetry(&mut harness);

    // Use a URI with identifying path components
    harness.open("file:///home/user/secret/project/main.pl", "my $x = ;")?;

    let notif = harness.wait_for_notification("telemetry/event", Duration::from_millis(800));
    assert!(notif.is_ok(), "Expected telemetry/event notification: {:?}", notif.err());

    let payload = notif?;
    let payload_str = serde_json::to_string(&payload)?;

    // File extension is acceptable (not PII)
    assert_eq!(payload["fileExtension"].as_str(), Some("pl"), "fileExtension should be 'pl'");

    // Path components must NOT appear in the payload
    for forbidden in &["/home/", "/user/", "secret", "project", "main.pl"] {
        assert!(
            !payload_str.contains(forbidden),
            "PII leak: payload contains '{}': {}",
            forbidden,
            payload_str
        );
    }

    // Source code must NOT appear in the payload
    assert!(
        !payload_str.contains("my $x"),
        "PII leak: payload contains source code: {}",
        payload_str
    );

    Ok(())
}
