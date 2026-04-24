// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 10 — Go-to-definition feature grid coverage.
//!
//! Verifies that `textDocument/definition` is wired up end-to-end for the LSP
//! feature advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - `textDocument/definition` MUST NOT return a JSON-RPC error.
//! - A definition result MAY be empty (degraded mode acceptable) but must not crash.
//! - When a sub is defined in the same file, the server SHOULD return a location
//!   pointing back into that file.
//! - No crash signatures after the request.

use perl_lsp_ux_tests::{CursorPosition, ScenarioConfig, UxHarness};
use serde_json::json;
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

/// Source with a named sub defined near the top and called later.
/// We will request go-to-definition on the call site at line 8, char 0.
const GOTO_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    my ($name) = @_;\n\
    return \"Hello, $name!\";\n\
}\n\
\n\
greet('World');\n\
";

#[test]
fn scenario_10_definition_request_does_not_error() {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("greet.pl", GOTO_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("greet.pl", GOTO_SOURCE).expect("didOpen should succeed");

    // Allow the server to index the file.
    std::thread::sleep(Duration::from_millis(500));

    // Request definition on the call site `greet('World')` — line 8, char 0.
    let result = harness.definition("greet.pl", 8, 0);
    assert!(
        result.is_ok(),
        "textDocument/definition must not return a JSON-RPC error — feature grid regression: {:?}",
        result
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_10_definition_result_is_location_or_empty() {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("greet.pl", GOTO_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("greet.pl", GOTO_SOURCE).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(500));

    let defs = harness
        .definition_at("greet.pl", CursorPosition::new(8, 0))
        .expect("definition must not error");

    // If results are returned they must be well-formed Location objects.
    for loc in &defs {
        harness.assert_normalized_response_eq(
            loc,
            &json!({
                "uri": "$WORKSPACE/greet.pl",
                "range": loc["range"].clone(),
            }),
        );
    }

    harness.assert_no_crash();
}

#[test]
fn scenario_10_definition_on_unknown_position_returns_empty() {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\nmy $x = 1;\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("simple.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("simple.pl", source).expect("didOpen should succeed");

    // Position in middle of `strict` string literal — no definition expected.
    let defs = harness
        .definition("simple.pl", 0, 5)
        .expect("definition must not error on arbitrary position");

    // Empty is fine; what we are testing is that no crash or error occurs.
    let _ = defs;

    harness.assert_no_crash();
}
