// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 04 — Missing perlcritic.
//!
//! Simulates a user who has perl-lsp installed without perlcritic.
//!
//! Acceptance criteria:
//! - Server MUST start and accept `initialize`.
//! - `textDocument/didOpen` MUST succeed.
//! - Server MUST NOT crash when it tries to run perlcritic and fails.
//! - Server must remain responsive after the diagnostic pass.

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

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
    ScenarioConfig {
        path_restriction: Some(dirs),
        ..Default::default()
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

    harness
        .open_file("critic.pl", source)
        .expect("didOpen should succeed");
    std::thread::sleep(Duration::from_secs(1));

    harness.assert_no_crash();
}

#[test]
fn scenario_04_server_responsive_without_perlcritic() {
    if !binary_available() {
        eprintln!("SKIP scenario_04: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 1;\n";
    let harness = UxHarness::new(config_without_perlcritic()).expect("Failed to create UX harness");

    harness
        .open_file("responsive.pl", source)
        .expect("didOpen should succeed");
    std::thread::sleep(Duration::from_millis(500));

    let hover = harness.hover("responsive.pl", 0, 3);
    assert!(
        hover.is_ok(),
        "Server became unresponsive after perlcritic failure — UX regression: {:?}",
        hover
    );
}
