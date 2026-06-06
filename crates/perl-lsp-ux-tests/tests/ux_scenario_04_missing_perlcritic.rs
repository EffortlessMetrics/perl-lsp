// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// UX receipt scenarios print skip markers during `--nocapture` runs.
#![allow(clippy::print_stderr)]

//! Scenario 04 — Missing perlcritic.
//!
//! Simulates a user who has perl-lsp installed without perlcritic.
//!
//! Acceptance criteria:
//! - Server MUST start and accept `initialize`.
//! - `textDocument/didOpen` MUST succeed.
//! - Server MUST NOT crash when it tries to run perlcritic and fails.
//! - Server must remain responsive after the diagnostic pass.

use anyhow::Result;
use perl_lsp_ux_tests::LspEvent;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::json;
use std::time::Duration;

fn config_without_perlcritic() -> ScenarioConfig {
    // Exclude only perlcritic from PATH, leaving perl and other tools available.
    // This accurately simulates "user has perl but not perlcritic installed".
    let sep = if cfg!(windows) { ';' } else { ':' };
    let dirs: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(sep)
        .filter(|entry| !entry.contains("perlcritic"))
        .map(String::from)
        .collect();
    ScenarioConfig { path_restriction: Some(dirs), ..Default::default() }
}

fn config_without_any_path_tools() -> ScenarioConfig {
    ScenarioConfig { path_restriction: Some(Vec::new()), ..Default::default() }
}

fn send_external_perlcritic_config(harness: &UxHarness) -> Result<()> {
    harness.client.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "perlcritic": {
                        "enabled": true
                    },
                    "critic": {
                        "engine": "perlcritic"
                    }
                }
            }
        }),
    )?;
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
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
fn scenario_04_diagnostics_without_perlcritic_no_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_04: perl-lsp binary not found");
        return;
    }

    let source = "sub foo {\n    my $unused = 1;\n    return 42;\n}\n";
    let harness = UxHarness::new(config_without_perlcritic()).expect("Failed to create UX harness");

    harness.open_file("critic.pl", source).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_secs(1));

    harness.assert_no_crash();
}

#[test]
fn scenario_04_external_perlcritic_missing_warning_is_actionable() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_04: perl-lsp binary not found");
        return Ok(());
    }

    let source = "sub foo {\n    my $unused = 1;\n    return 42;\n}\n";
    let harness = UxHarness::new(config_without_any_path_tools())?;
    send_external_perlcritic_config(&harness)?;

    harness.open_file("external_critic_missing.pl", source)?;
    std::thread::sleep(Duration::from_secs(1));

    let events = harness.peek_notifications();
    let warning = events
        .iter()
        .filter_map(message_text)
        .find(|message| message.contains("`perlcritic` was not found"));
    let Some(warning) = warning else {
        anyhow::bail!("external perlcritic should emit a missing-tool warning; events={events:?}");
    };

    assert!(
        warning.contains("cpanm Perl::Critic"),
        "missing-tool warning should include install guidance: {warning}"
    );
    assert!(
        warning.contains("disable perl.perlcritic.enabled"),
        "missing-tool warning should include disable guidance: {warning}"
    );
    assert!(
        !warning.contains("panicked at") && !warning.contains("SIGABRT"),
        "missing-tool warning should not expose crash text: {warning}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_04_server_responsive_without_perlcritic() {
    if !binary_available() {
        eprintln!("SKIP scenario_04: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 1;\n";
    let harness = UxHarness::new(config_without_perlcritic()).expect("Failed to create UX harness");

    harness.open_file("responsive.pl", source).expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let hover = harness.hover("responsive.pl", 0, 3);
    assert!(
        hover.is_ok(),
        "Server became unresponsive after perlcritic failure — UX regression: {:?}",
        hover
    );
}
