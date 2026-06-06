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
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde_json::Value;

const SCENARIO_FILE: &str = "ux_scenario_19_completion_core.rs";

const COMPLETION_FIXTURE: &str = r#"use strict;
use warnings;

pri

my $value = 42;
my $display = $val
"#;

fn create_harness() -> Result<UxHarness> {
    UxHarness::new(ScenarioConfig::default().with_file("completion.pl", COMPLETION_FIXTURE))
}

fn completion_item_has_display_shape(item: &Value) -> bool {
    item.get("label").and_then(Value::as_str).is_some()
        || item.get("insertText").and_then(Value::as_str).is_some()
        || item.get("filterText").and_then(Value::as_str).is_some()
}

fn completion_items_include_label(items: &[Value], needle: &str) -> bool {
    items
        .iter()
        .filter_map(|item| item.get("label").and_then(Value::as_str))
        .any(|label| label.contains(needle))
}

#[test]
fn scenario_19_completion_request_does_not_error() {
    run_ux_scenario(
        "completion_core",
        SCENARIO_FILE,
        "scenario_19_completion_request_does_not_error",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file("completion.pl", COMPLETION_FIXTURE)?;

            recorder.mark_request_start("completion_builtin_prefix");
            let completion_result = harness.completion("completion.pl", 3, 3);
            recorder.check(
                "textDocument/completion for `pri` does not return a JSON-RPC error",
                completion_result.is_ok(),
            )?;
            let items = completion_result?;
            if !items.is_empty() {
                recorder.mark_first_useful_result("completion_builtin_prefix");
            }
            recorder.check(
                "completion returns at least one item for `pri` prefix",
                !items.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_19_completion_items_have_label_or_insert_text_shape() {
    run_ux_scenario(
        "completion_core",
        SCENARIO_FILE,
        "scenario_19_completion_items_have_label_or_insert_text_shape",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file("completion.pl", COMPLETION_FIXTURE)?;

            recorder.mark_request_start("completion_scalar_prefix");
            let completion_result = harness.completion("completion.pl", 6, 18);
            recorder.check(
                "textDocument/completion for `$val` does not return a JSON-RPC error",
                completion_result.is_ok(),
            )?;
            let items = completion_result?;
            if !items.is_empty() {
                recorder.mark_first_useful_result("completion_scalar_prefix");
            }
            recorder.check(
                "completion returns at least one item for `$val` prefix with `$value` in scope",
                !items.is_empty(),
            )?;
            recorder.check(
                "every completion item includes a user-visible string field",
                items.iter().all(completion_item_has_display_shape),
            )?;
            recorder.check(
                "completion labels include `value` for `$val` prefix",
                completion_items_include_label(&items, "value"),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}

#[test]
fn scenario_19_completion_builtin_workflow_surfaces_print() {
    run_ux_scenario(
        "completion_core",
        SCENARIO_FILE,
        "scenario_19_completion_builtin_workflow_surfaces_print",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = create_harness()?;
            harness.open_file("completion.pl", COMPLETION_FIXTURE)?;

            recorder.mark_request_start("completion_print_builtin");
            let labels = harness.completion_labels("completion.pl", 3, 3)?;
            let includes_print = labels.iter().any(|label| label == "print");
            if includes_print {
                recorder.mark_first_useful_result("completion_print_builtin");
            }
            recorder
                .check("builtin completion includes `print` for `pri` prefix", includes_print)?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
