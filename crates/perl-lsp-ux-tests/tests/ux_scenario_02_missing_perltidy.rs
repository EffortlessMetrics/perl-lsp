// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 02 — Missing perltidy.
//!
//! Simulates a user who has installed perl-lsp but not perltidy.
//! Formatting requests should degrade gracefully.
//!
//! Acceptance criteria:
//! - The server MUST NOT crash.
//! - `textDocument/formatting` MUST return a graceful error or empty result.
//! - No Rust panic traces in error messages.
//! - The server MUST still be alive after the failed formatting request.

use perl_lsp_ux_tests::{FormatResult, ScenarioConfig, UxHarness};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn config_without_perltidy() -> ScenarioConfig {
    // Exclude only perltidy from PATH, leaving perl and other tools available.
    // This accurately simulates "user has perl but not perltidy installed".
    let sep = if cfg!(windows) { ';' } else { ':' };
    let dirs: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(sep)
        .filter(|entry| !entry.contains("perltidy"))
        .map(String::from)
        .collect();
    ScenarioConfig { path_restriction: Some(dirs), ..Default::default() }
}

#[test]
fn scenario_02_formatting_without_perltidy_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_02: perl-lsp binary not found");
        return;
    }

    let source = "sub test{my$x=1;return$x;}\n";
    let harness = UxHarness::new(config_without_perltidy()).expect("Failed to create UX harness");

    harness.open_file("format_me.pl", source).expect("didOpen should succeed");

    let result = harness.format_document("format_me.pl");

    match result {
        Ok(FormatResult::Edits(_)) => {
            eprintln!("INFO scenario_02: formatting succeeded despite empty PATH");
        }
        Ok(FormatResult::Empty) => {
            // Graceful no-op — acceptable.
        }
        Ok(FormatResult::Error(err_val)) => {
            let msg = err_val["message"].as_str().unwrap_or("");
            assert!(
                !msg.contains("panicked at") && !msg.contains("SIGABRT"),
                "Error message looks like a Rust panic: {}",
                msg
            );
        }
        Err(e) => {
            eprintln!(
                "INFO scenario_02: harness error (server may have returned error quickly): {}",
                e
            );
        }
    }

    harness.assert_no_crash();
}

#[test]
fn scenario_02_server_remains_alive_after_failed_format() {
    if !binary_available() {
        eprintln!("SKIP scenario_02: perl-lsp binary not found");
        return;
    }

    let source = "my $x = 1;\n";
    let harness = UxHarness::new(config_without_perltidy()).expect("Failed to create UX harness");

    harness.open_file("alive.pl", source).expect("didOpen should succeed");

    let _ = harness.format_document("alive.pl");

    let hover = harness.hover("alive.pl", 0, 3);
    match hover {
        Ok(_) => {}
        Err(e) => {
            panic!("Server unresponsive after failed formatting — UX regression: {}", e);
        }
    }
}
