// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 01 — Clean install, simple file.
//!
//! Simulates the very first thing a user does after installing perl-lsp:
//! open a trivial `.pl` file and verify the server responds to hover.
//!
//! Acceptance criteria:
//! - Server starts without crashing.
//! - `textDocument/didOpen` is accepted (no error).
//! - `textDocument/hover` on a variable returns something, or null in degraded mode.
//! - No crash signatures in the event log.

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[test]
fn scenario_01_server_starts_and_accepts_open() {
    if !binary_available() {
        eprintln!("SKIP scenario_01: perl-lsp binary not found");
        return;
    }

    let source = "#!/usr/bin/env perl\nuse strict;\n\nprint \"Hello, world!\\n\";\n";

    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("hello.pl", source).expect("textDocument/didOpen should succeed");

    harness.assert_no_crash();
}

#[test]
fn scenario_01_hover_on_simple_variable() {
    if !binary_available() {
        eprintln!("SKIP scenario_01: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\nuse warnings;\n\nmy $x = 42;\nmy $y = $x + 1;\n";

    let harness = UxHarness::new(ScenarioConfig::default().with_file("test.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("test.pl", source).expect("textDocument/didOpen should not fail");

    // Hover on `$x` (line 3, character 3).
    let hover_result = harness.hover("test.pl", 3, 3);

    match hover_result {
        Ok(Some(result)) => {
            assert!(
                result.is_object() || result.is_string(),
                "Hover result should be an object, got: {:?}",
                result
            );
        }
        Ok(None) => {
            eprintln!("INFO scenario_01: hover returned null (degraded mode acceptable)");
        }
        Err(e) => {
            panic!("Hover returned a JSON-RPC error — this is a UX regression: {}", e);
        }
    }

    harness.assert_no_crash();
}

#[test]
fn scenario_01_completion_on_keyword_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_01: perl-lsp binary not found");
        return;
    }

    let source = "use str\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("complete.pl", source).expect("didOpen should succeed");

    let result = harness.completion("complete.pl", 0, 7);
    assert!(result.is_ok(), "completion should not crash: {:?}", result);
}
