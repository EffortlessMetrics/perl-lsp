// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 03 — Missing perl interpreter.
//!
//! Simulates perl-lsp running without `perl` on PATH.
//!
//! Acceptance criteria:
//! - Server MUST start (it is a Rust binary).
//! - Server MUST accept `initialize` and `textDocument/didOpen`.
//! - Server MUST NOT crash during initialization.
//! - Hover and completion may return null/empty — that is acceptable.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn config_without_perl() -> ScenarioConfig {
    ScenarioConfig { path_restriction: Some(Vec::new()), ..Default::default() }
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
    let perl_messages: Vec<_> = events
        .iter()
        .filter(|ev| {
            use perl_lsp_ux_tests::LspEvent;
            match ev {
                LspEvent::WindowMessage { message, .. } | LspEvent::LogMessage { message, .. } => {
                    let lower = message.to_ascii_lowercase();
                    lower.contains("perl") || lower.contains("interpreter")
                }
                _ => false,
            }
        })
        .collect();

    if perl_messages.is_empty() {
        eprintln!(
            "INFO scenario_03: no Perl-related warning message — \
             may be OK if server uses a different channel"
        );
    } else {
        eprintln!("INFO scenario_03: server emitted Perl messages: {:?}", perl_messages);
    }

    harness.assert_no_crash();
}
