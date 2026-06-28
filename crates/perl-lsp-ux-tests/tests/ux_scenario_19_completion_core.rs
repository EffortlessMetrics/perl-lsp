//! Scenario 19 — Completion UX depth coverage.
//!
//! Focuses on first-session completion ergonomics using a representative Perl
//! source file and targeted cursor positions.
//!
//! Acceptance criteria:
//! - `textDocument/completion` MUST NOT return a JSON-RPC error.
//! - Completion items (when present) MUST expose a usable display shape.
//! - Built-in completion workflows SHOULD include `print` for `pri` prefix.
//! - No crash signatures after repeated completion requests.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};

const COMPLETION_FIXTURE: &str = r#"use strict;
use warnings;

pri

my $value = 42;
my $display = $val
"#;

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(ScenarioConfig::default().with_file("completion.pl", COMPLETION_FIXTURE))
}

#[test]
fn scenario_19_completion_request_does_not_error() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    // Given a file opened in the editor.
    let harness = create_harness()?;
    harness.open_file("completion.pl", COMPLETION_FIXTURE)?;

    // When requesting completion for an in-progress builtin (`pri`).
    let items = harness
        .completion("completion.pl", 3, 3)
        .map_err(|e| anyhow::anyhow!("textDocument/completion returned JSON-RPC error: {e}"))?;

    // Then the request succeeds and returns a non-empty payload — `pri` is a
    // prefix of the `print`/`printf` builtins so at least one completion item
    // must be surfaced. An empty list here is a real regression, not a
    // degraded-mode pass.
    assert!(
        !items.is_empty(),
        "expected at least one completion item for `pri` prefix, got empty list"
    );
    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_19_completion_items_have_label_or_insert_text_shape() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    // Given a file opened in the editor.
    let harness = create_harness()?;
    harness.open_file("completion.pl", COMPLETION_FIXTURE)?;

    // When requesting completion near a scalar variable prefix (`$val`).
    let items = harness.completion("completion.pl", 6, 18)?;

    // Then the server returns at least one item — `$value` was declared on the
    // previous line so a completion-driven editor must surface something.
    // (Vacuously passing on an empty list defeats the purpose of the test.)
    assert!(
        !items.is_empty(),
        "expected at least one completion item for `$val` prefix with `$value` in scope"
    );

    // And every returned item has a user-visible completion field.
    for item in &items {
        let has_label = item.get("label").and_then(serde_json::Value::as_str).is_some();
        let has_insert_text = item.get("insertText").and_then(serde_json::Value::as_str).is_some();
        let has_filter_text = item.get("filterText").and_then(serde_json::Value::as_str).is_some();
        assert!(
            has_label || has_insert_text || has_filter_text,
            "completion item must include a string label, insertText, or filterText: {item:?}"
        );
    }

    // And at least one item surfaces the `$value` identifier the user started
    // typing — this is the behaviour the UX depends on.
    let labels = harness.completion_labels("completion.pl", 6, 18)?;
    assert!(
        labels.iter().any(|label| label.contains("value")),
        "expected completion label containing `value` for `$val` prefix, got: {labels:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_19_completion_builtin_workflow_surfaces_print() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    // Given a file opened in the editor.
    let harness = create_harness()?;
    harness.open_file("completion.pl", COMPLETION_FIXTURE)?;

    // When requesting completion after typing `pri`.
    let labels = harness.completion_labels("completion.pl", 3, 3)?;

    // Then `print` appears in suggestions, proving a practical UX path.
    assert!(
        labels.iter().any(|label| label == "print"),
        "expected builtin `print` in completion suggestions, got labels: {labels:?}"
    );

    harness.assert_no_crash();
    Ok(())
}
