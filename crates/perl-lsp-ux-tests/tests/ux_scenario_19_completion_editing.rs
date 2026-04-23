//! Scenario 19 — completion UX during active editing.
//!
//! Drives a realistic editor loop:
//! 1. Open a file.
//! 2. Request completion by explicit invoke.
//! 3. Edit the document (`didChange`).
//! 4. Request completion again with trigger-character and incomplete-result modes.
//!
//! Acceptance criteria:
//! - Completion requests succeed across trigger modes.
//! - `didChange` does not destabilize subsequent completion requests.
//! - Returned completion payloads stay structurally valid.

use anyhow::Result;
use perl_lsp_ux_tests::{CompletionTrigger, ScenarioConfig, UxHarness};
use serde_json::Value;

const INITIAL_SOURCE: &str = "use strict;\nuse warnings;\n\nmy $alpha = 1;\nmy $value = $al\n";
const UPDATED_SOURCE: &str = "use strict;\nuse warnings;\n\nmy $alpha = 1;\nmy $value = $alp\n";

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn assert_completion_items_have_labels(items: &[Value], phase: &str) {
    assert!(
        !items.is_empty(),
        "{phase}: completion returned no items — a `$al`/`$alp` prefix with \
         `$alpha` in scope must surface at least one item; vacuous pass would \
         hide a real regression."
    );
    for item in items {
        // Per LSP spec, completion items are objects. Null entries indicate a
        // malformed payload, not a tolerated degraded case.
        assert!(
            !item.is_null(),
            "{phase}: completion item is null, expected CompletionItem object"
        );
        assert!(
            item.get("label").and_then(Value::as_str).is_some(),
            "{phase}: completion item missing string label: {item:?}"
        );
    }
}

fn labels_contain(items: &[Value], needle: &str) -> bool {
    items.iter().any(|item| {
        item.get("label")
            .and_then(Value::as_str)
            .map(|label| label.contains(needle))
            .unwrap_or(false)
    })
}

#[test]
fn scenario_19_completion_remains_stable_across_edit_and_trigger_modes() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig::default().with_file("editing.pl", INITIAL_SOURCE))?;

    // Given an opened Perl document.
    harness.open_file("editing.pl", INITIAL_SOURCE)?;

    // When completion is manually invoked after typing `$al`.
    let initial_items =
        harness.completion_with_trigger("editing.pl", 4, 16, CompletionTrigger::Invoked)?;
    assert_completion_items_have_labels(&initial_items, "initial invoke completion");
    // And the `$alpha` identifier declared on the prior line is surfaced —
    // this is the concrete UX behaviour the test claims to protect.
    assert!(
        labels_contain(&initial_items, "alpha"),
        "initial invoke completion: expected label containing `alpha`, got labels: {:?}",
        initial_items
            .iter()
            .filter_map(|i| i.get("label").and_then(Value::as_str))
            .collect::<Vec<_>>()
    );

    // And the user continues typing in the same line.
    harness.apply_full_change("editing.pl", UPDATED_SOURCE, 2)?;

    // Then trigger-character completion remains healthy.
    let triggered_items = harness.completion_with_trigger(
        "editing.pl",
        4,
        17,
        CompletionTrigger::TriggerCharacter('p'),
    )?;
    assert_completion_items_have_labels(&triggered_items, "trigger-character completion");
    // And `$alpha` is still a valid completion after the edit — proves the
    // didChange did not desync the server's view of the document.
    assert!(
        labels_contain(&triggered_items, "alpha"),
        "trigger-character completion: expected `alpha` after didChange; server \
         view of document may be stale"
    );

    // And follow-up incomplete completion requests also remain healthy.
    let incomplete_items = harness.completion_with_trigger(
        "editing.pl",
        4,
        17,
        CompletionTrigger::TriggerForIncompleteCompletions,
    )?;
    assert_completion_items_have_labels(&incomplete_items, "incomplete completion");

    harness.assert_no_crash();
    Ok(())
}
