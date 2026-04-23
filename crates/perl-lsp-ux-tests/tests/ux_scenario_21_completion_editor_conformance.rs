//! Scenario 21 — Cross-editor completion conformance.
//!
//! Emulates completion capability probes from VS Code, Zed, Neovim, and Helix
//! and verifies we return a stable completion contract across profiles for
//! snippet and commit-character metadata.

use anyhow::Result;
use perl_lsp_ux_tests::{EditorClientProfile, ScenarioConfig, UxHarness};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
struct CompletionSignature {
    label: String,
    kind: Option<u64>,
    insert_text_format: Option<u64>,
    has_snippet_tabstop: bool,
    commit_characters: Vec<String>,
}

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn capture_signatures(profile: EditorClientProfile) -> Result<Vec<CompletionSignature>> {
    let source = "su\n";
    let harness = UxHarness::new(
        ScenarioConfig::default().with_editor_profile(profile).with_file("main.pl", source),
    )?;
    harness.open_file("main.pl", source)?;

    let items = harness.completion("main.pl", 0, 2)?;

    let mut out = items
        .into_iter()
        .map(|item| to_signature(item).unwrap_or_default())
        .filter(|sig| !sig.label.is_empty())
        .collect::<Vec<_>>();

    out.sort();
    Ok(out)
}

fn to_signature(item: Value) -> Option<CompletionSignature> {
    let label = item.get("label")?.as_str()?.to_string();
    let kind = item.get("kind").and_then(Value::as_u64);
    let insert_text = item.get("insertText").and_then(Value::as_str).unwrap_or_default();
    let insert_text_format = item.get("insertTextFormat").and_then(Value::as_u64);

    let commit_characters = item
        .get("commitCharacters")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Some(CompletionSignature {
        label,
        kind,
        insert_text_format,
        has_snippet_tabstop: insert_text.contains("$0") || insert_text.contains("${1:"),
        commit_characters,
    })
}

impl Default for CompletionSignature {
    fn default() -> Self {
        Self {
            label: String::new(),
            kind: None,
            insert_text_format: None,
            has_snippet_tabstop: false,
            commit_characters: Vec::new(),
        }
    }
}

#[test]
fn scenario_21_completion_metadata_stays_consistent_across_editors() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_21: perl-lsp binary not found");
        return Ok(());
    }

    let profiles = [
        EditorClientProfile::VsCode,
        EditorClientProfile::Zed,
        EditorClientProfile::Neovim,
        EditorClientProfile::Helix,
    ];

    let mut signatures_by_editor = BTreeMap::new();
    for profile in profiles {
        signatures_by_editor.insert(profile.as_str().to_string(), capture_signatures(profile)?);
    }

    let baseline = signatures_by_editor.get("vscode").cloned().unwrap_or_default();

    for (editor, signatures) in &signatures_by_editor {
        assert_eq!(signatures, &baseline, "completion signature drifted for {editor} vs vscode");
    }

    assert!(
        baseline.iter().any(|sig| sig.has_snippet_tabstop),
        "expected at least one snippet-style completion with tab stops"
    );

    Ok(())
}
