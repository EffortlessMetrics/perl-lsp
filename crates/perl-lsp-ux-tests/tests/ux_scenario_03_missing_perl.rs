// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// UX receipt scenarios print skip markers during `--nocapture` runs.
#![allow(clippy::print_stderr)]

//! Scenario 03 — Missing perl interpreter.
//!
//! Simulates perl-lsp running without `perl` on PATH.
//!
//! Acceptance criteria:
//! - Server MUST start (it is a Rust binary).
//! - Server MUST accept `initialize` and `textDocument/didOpen`.
//! - Server MUST NOT crash during initialization.
//! - Hover and completion may return null/empty — that is acceptable.

use perl_lsp_ux_tests::LspEvent;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn config_without_perl() -> ScenarioConfig {
    ScenarioConfig { path_restriction: Some(Vec::new()), ..Default::default() }
}

fn message_text(event: &LspEvent) -> Option<&str> {
    match event {
        LspEvent::WindowMessage { message, .. } | LspEvent::LogMessage { message, .. } => {
            Some(message.as_str())
        }
        _ => None,
    }
}

#[test]
fn scenario_03_server_starts_without_perl() {
    if !binary_available() {
        eprintln!("SKIP scenario_03: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\nmy $x = 1;\n";
    let harness =
        UxHarness::new(config_without_perl()).expect("Failed to create UX harness (no perl)");

    harness.open_file("no_perl.pl", source).expect("didOpen should succeed without perl");

    harness.assert_no_crash();
}

#[test]
fn scenario_03_degraded_mode_hover_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_03: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 42;\n";
    let harness =
        UxHarness::new(config_without_perl()).expect("Failed to create UX harness (no perl)");

    harness.open_file("degraded.pl", source).expect("didOpen should succeed");

    let result = harness.hover("degraded.pl", 0, 3);
    assert!(
        result.is_ok(),
        "hover should not return transport error in degraded mode: {:?}",
        result
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_03_degraded_mode_completion_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_03: perl-lsp binary not found");
        return;
    }

    let source = "use str\n";
    let harness =
        UxHarness::new(config_without_perl()).expect("Failed to create UX harness (no perl)");

    harness.open_file("complete.pl", source).expect("didOpen should succeed");

    let result = harness.completion("complete.pl", 0, 7);
    assert!(result.is_ok(), "completion should not error in degraded mode: {:?}", result);

    harness.assert_no_crash();
}

#[test]
fn scenario_03_warning_message_about_missing_perl() {
    if !binary_available() {
        eprintln!("SKIP scenario_03: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 1;\n";
    let harness =
        UxHarness::new(config_without_perl()).expect("Failed to create UX harness (no perl)");

    harness.open_file("warn_test.pl", source).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_secs(1));

    let events = harness.collect_notifications();
    let fallback_message = events.iter().filter_map(message_text).find(|message| {
        message.contains("Perl not found on PATH") || message.contains("Perl interpreter not found")
    });
    let Some(fallback_message) = fallback_message else {
        panic!("missing-Perl startup should emit actionable setup guidance; events={events:?}");
    };

    assert!(
        fallback_message.contains("Add Perl to PATH")
            || fallback_message.contains("Install Perl")
            || fallback_message.contains("install Perl"),
        "missing-Perl message should include install/PATH guidance: {fallback_message}"
    );
    assert!(
        fallback_message.contains("perl-lsp.perl.path"),
        "missing-Perl message should name the explicit interpreter setting: {fallback_message}"
    );
    assert!(
        !fallback_message.contains("panicked at") && !fallback_message.contains("SIGABRT"),
        "missing-Perl message should not expose crash text: {fallback_message}"
    );

    harness.assert_no_crash();
}
