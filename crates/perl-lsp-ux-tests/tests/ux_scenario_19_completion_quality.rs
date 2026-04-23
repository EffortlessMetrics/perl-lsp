// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — completion quality for first-file editing.
//!
//! Verifies that completion is not only crash-free, but returns actionable
//! suggestions for a high-frequency typing path (`pri` -> `print`).

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[test]
fn scenario_19_completion_returns_actionable_builtin_suggestions() {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\nuse warnings;\n\npri\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("completion.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("completion.pl", source).expect("didOpen should succeed");

    let completion_items =
        harness.completion("completion.pl", 3, 3).expect("completion request should not error");

    assert!(
        !completion_items.is_empty(),
        "Expected completion to return at least one suggestion for `pri`, got empty list"
    );

    let has_print = completion_items.iter().any(|item| {
        item.get("label")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|label| label == "print" || label.starts_with("print "))
    });

    assert!(
        has_print,
        "Expected completion suggestions for `pri` to include `print`, got: {:?}",
        completion_items
    );

    harness.assert_no_crash();
}
