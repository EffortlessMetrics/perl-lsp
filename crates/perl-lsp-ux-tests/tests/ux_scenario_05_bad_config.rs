// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// UX receipt scenarios print skip markers during `--nocapture` runs.
#![allow(clippy::print_stderr)]

//! Scenario 05 — Bad configuration.
//!
//! Simulates a user who has set invalid values in their configuration.
//!
//! Acceptance criteria:
//! - Server MUST NOT crash on startup with invalid config.
//! - Error messages MUST NOT contain raw Rust panic traces.
//! - Server MUST remain responsive after the config error.

use anyhow::Result;
use perl_lsp_ux_tests::LspEvent;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn config_with_bad_tool_paths() -> ScenarioConfig {
    ScenarioConfig::default()
        .env("PERLTIDY_PATH", "/nonexistent/path/to/perltidy")
        .env("PERLCRITIC_PATH", "/nonexistent/path/to/perlcritic")
}

fn config_with_invalid_project_toml() -> ScenarioConfig {
    ScenarioConfig::default().with_file(".perl-lsp.toml", "[perl\ninclude_paths = [\"lib\"]")
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
fn scenario_05_bad_tool_path_does_not_crash_server() {
    if !binary_available() {
        eprintln!("SKIP scenario_05: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 1;\n";
    let harness = UxHarness::new(config_with_bad_tool_paths())
        .expect("Failed to create UX harness with bad config");

    harness.open_file("config_test.pl", source).expect("didOpen should succeed");

    std::thread::sleep(std::time::Duration::from_millis(500));

    harness.assert_no_crash();
}

#[test]
fn scenario_05_server_responsive_with_bad_config() {
    if !binary_available() {
        eprintln!("SKIP scenario_05: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 1;\nmy $y = $x + 1;\n";
    let harness =
        UxHarness::new(config_with_bad_tool_paths()).expect("Failed to create UX harness");

    harness.open_file("config_responsive.pl", source).expect("didOpen should succeed");

    let fmt = harness.format_document("config_responsive.pl");
    let hover = harness.hover("config_responsive.pl", 0, 3);

    // At least one of these must succeed — server must still be alive.
    assert!(
        hover.is_ok() || fmt.is_ok(),
        "Server became unresponsive with bad config — UX regression. hover={:?} fmt={:?}",
        hover,
        fmt
    );
}

#[test]
fn scenario_05_format_with_bad_perltidy_path_returns_graceful_error() {
    if !binary_available() {
        eprintln!("SKIP scenario_05: perl-lsp binary not found");
        return;
    }

    let source = "sub foo{my$x=1;}\n";
    let harness =
        UxHarness::new(config_with_bad_tool_paths()).expect("Failed to create UX harness");

    harness.open_file("format_bad.pl", source).expect("didOpen should succeed");

    match harness.format_document("format_bad.pl") {
        Ok(result) => {
            if result.is_error() {
                let msg = result.error_message().unwrap_or("");
                assert!(
                    !msg.contains("panicked at"),
                    "Error message contains Rust panic trace — UX regression: {}",
                    msg
                );
            }
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                !msg.contains("panicked at"),
                "Formatting error contains Rust panic trace — UX regression: {}",
                msg
            );
        }
    }
}

#[test]
fn scenario_05_invalid_project_config_warning_is_actionable() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_05: perl-lsp binary not found");
        return Ok(());
    }

    let source = "my $x = 1;\n";
    let harness = UxHarness::new(config_with_invalid_project_toml())?;

    harness.open_file("invalid_config.pl", source)?;
    std::thread::sleep(Duration::from_millis(500));

    let events = harness.peek_notifications();
    let warning = events
        .iter()
        .filter_map(message_text)
        .find(|message| message.contains(".perl-lsp.toml") && message.contains("syntax error"));
    let Some(warning) = warning else {
        anyhow::bail!(
            "invalid .perl-lsp.toml should emit actionable syntax warning; events={events:?}"
        );
    };

    assert!(
        warning.contains("Fix the error in .perl-lsp.toml"),
        "config warning should tell the user what to fix: {warning}"
    );
    assert!(
        warning.contains("reload the window"),
        "config warning should tell the user how to apply the fix: {warning}"
    );
    assert!(
        !warning.contains("panicked at") && !warning.contains("SIGABRT"),
        "config warning should not expose crash text: {warning}"
    );

    harness.assert_no_crash();
    Ok(())
}
