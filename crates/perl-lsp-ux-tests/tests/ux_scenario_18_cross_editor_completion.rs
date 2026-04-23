//! Scenario 18 — Cross-editor completion parity.
//!
//! Editors probe completion with different trigger contexts. This scenario
//! verifies we keep stable completion payloads across representative editor
//! request shapes (VS Code, Zed, Neovim, Helix).

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[derive(Clone, Copy)]
struct EditorProbe {
    name: &'static str,
    trigger_kind: u32,
    trigger_character: Option<&'static str>,
}

const EDITOR_PROBES: [EditorProbe; 4] = [
    EditorProbe { name: "vscode", trigger_kind: 1, trigger_character: None },
    EditorProbe { name: "zed", trigger_kind: 2, trigger_character: Some("$") },
    EditorProbe { name: "neovim", trigger_kind: 3, trigger_character: Some("$") },
    EditorProbe { name: "helix", trigger_kind: 1, trigger_character: None },
];

#[test]
fn scenario_18_cross_editor_completion_payloads_stay_aligned() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18: perl-lsp binary not found");
        return Ok(());
    }

    let source = "use strict;\nuse warnings;\n\nmy $alpha = 41;\nmy $beta = $alp\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("cross_editor.pl", source))
        .context("Failed to create UX harness")?;

    harness
        .open_file("cross_editor.pl", source)
        .context("didOpen should succeed for cross-editor scenario")?;

    let mut baseline_shape: Option<(Option<String>, Option<u64>, Option<Vec<String>>)> = None;

    for probe in EDITOR_PROBES {
        let items = harness
            .completion_with_context(
                "cross_editor.pl",
                4,
                16,
                probe.trigger_kind,
                probe.trigger_character,
            )
            .with_context(|| format!("completion request failed for {}", probe.name))?;

        assert!(
            !items.is_empty(),
            "{} probe should return at least one completion item",
            probe.name
        );

        let alpha_item = find_item_by_label(&items, "$alpha").with_context(|| {
            format!("{} probe should include lexical completion for $alpha", probe.name)
        })?;

        let current_shape = completion_shape(alpha_item);
        if let Some(expected_shape) = &baseline_shape {
            assert_eq!(
                &current_shape, expected_shape,
                "{} probe returned divergent completion payload shape for $alpha",
                probe.name
            );
        } else {
            baseline_shape = Some(current_shape);
        }
    }

    harness.assert_no_crash();
    Ok(())
}

fn find_item_by_label<'a>(items: &'a [Value], label: &str) -> Result<&'a Value> {
    items
        .iter()
        .find(|item| item.get("label").and_then(Value::as_str) == Some(label))
        .with_context(|| format!("completion item with label `{label}` missing"))
}

fn completion_shape(item: &Value) -> (Option<String>, Option<u64>, Option<Vec<String>>) {
    let insert_text = item.get("insertText").and_then(Value::as_str).map(str::to_string);
    let insert_text_format = item.get("insertTextFormat").and_then(Value::as_u64);
    let commit_characters = item.get("commitCharacters").and_then(Value::as_array).map(|chars| {
        chars.iter().filter_map(Value::as_str).map(str::to_string).collect::<Vec<_>>()
    });

    (insert_text, insert_text_format, commit_characters)
}
