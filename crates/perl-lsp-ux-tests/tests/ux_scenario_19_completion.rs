// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — Completion quality in first-edit workflows.
//!
//! This scenario targets a high-impact UX path: users typing code and relying on
//! completion to discover local variables and common builtins.
//!
//! Acceptance criteria:
//! - `textDocument/completion` MUST NOT return a JSON-RPC error.
//! - Local variable completion should include the expected lexical symbol.
//! - Builtin/keyword completion should return a well-formed item list.
//! - No crash signatures after completion requests.

use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[test]
fn scenario_19_completion_surfaces_local_variables() {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\nuse warnings;\n\nmy $result = 42;\nmy $response = 100;\n\n$res\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("completion.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("completion.pl", source).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(300));

    // Completion on `$res` (line 6, char 4) should include lexical symbols.
    let items = harness
        .completion("completion.pl", 6, 4)
        .expect("completion should not return JSON-RPC error");

    let labels = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|label| label.as_str()))
        .collect::<Vec<_>>();

    assert!(
        labels.contains(&"$result"),
        "Expected lexical completion to include $result, got labels: {:?}",
        labels
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_19_completion_items_have_useful_shape_for_builtins() {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\n\npri\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("builtins.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("builtins.pl", source).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(300));

    let items = harness
        .completion("builtins.pl", 2, 3)
        .expect("completion should not return JSON-RPC error");

    assert!(!items.is_empty(), "Expected non-empty completion items for `pri`");

    for item in &items {
        assert!(
            item.get("label").and_then(|label| label.as_str()).is_some(),
            "Completion items must include a string `label`, got: {:?}",
            item
        );
    }

    let labels = items
        .iter()
        .filter_map(|item| item.get("label").and_then(|label| label.as_str()))
        .collect::<Vec<_>>();

    assert!(
        labels.iter().any(|label| label.contains("print")),
        "Expected builtin-like completion for `pri` to include `print`, got labels: {:?}",
        labels
    );

    harness.assert_no_crash();
}
