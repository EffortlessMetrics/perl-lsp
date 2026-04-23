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
    for item in items {
        if item.is_null() {
            continue;
        }
        assert!(
            item.get("label").and_then(Value::as_str).is_some(),
            "{phase}: completion item missing string label: {item:?}"
        );
    }
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

    // When completion is manually invoked.
    let initial_items =
        harness.completion_with_trigger("editing.pl", 4, 16, CompletionTrigger::Invoked)?;
    assert_completion_items_have_labels(&initial_items, "initial invoke completion");

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
