// Test infrastructure — allow scenario-oriented assertions and skips.
#![allow(clippy::expect_used, clippy::panic)]

//! Scenario 19 — fixture-backed editor-intelligence scorecard smoke.
//!
//! This scenario runs a single JSON fixture that exercises the canonical LSP UX
//! request/response surface in one place (hover/completion/navigation/symbols/
//! rename/diagnostics-after-edit) and emits simple measured rates.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

const FIXTURE_JSON: &str = include_str!("../fixtures/lsp_editor_intelligence_fixture.json");

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct PositionCheck {
    path: String,
    line: u32,
    character: u32,
}

#[derive(Debug, Deserialize)]
struct HoverCheck {
    #[serde(flatten)]
    position: PositionCheck,
    contains: String,
}

#[derive(Debug, Deserialize)]
struct CompletionCheck {
    #[serde(flatten)]
    position: PositionCheck,
    must_include: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NavCheck {
    #[serde(flatten)]
    position: PositionCheck,
    target_suffix: String,
    min_results: usize,
}

#[derive(Debug, Deserialize)]
struct ReferencesCheck {
    #[serde(flatten)]
    position: PositionCheck,
    min_results: usize,
    include_declaration: bool,
}

#[derive(Debug, Deserialize)]
struct DocumentSymbolsCheck {
    path: String,
    must_include: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSymbolsCheck {
    query: String,
    min_results: usize,
}

#[derive(Debug, Deserialize)]
struct RenameCheck {
    #[serde(flatten)]
    position: PositionCheck,
    new_name: String,
    min_edits: usize,
}

#[derive(Debug, Deserialize)]
struct DiagnosticsCheck {
    path: String,
    after_open_min: usize,
    after_fix_max: usize,
    fixed_content: String,
}

#[derive(Debug, Deserialize)]
struct Checks {
    hover: HoverCheck,
    completion: CompletionCheck,
    definition: NavCheck,
    declaration: NavCheck,
    references: ReferencesCheck,
    document_symbols: DocumentSymbolsCheck,
    workspace_symbols: WorkspaceSymbolsCheck,
    rename: RenameCheck,
    diagnostics: DiagnosticsCheck,
}

#[derive(Debug, Deserialize)]
struct EditorFixture {
    schema_version: u32,
    workspace_files: Vec<FixtureFile>,
    checks: Checks,
}

#[test]
fn scenario_18_fixture_backed_editor_intelligence_surface() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_18: perl-lsp binary not found");
        return Ok(());
    }

    let fixture: EditorFixture =
        serde_json::from_str(FIXTURE_JSON).context("parse fixture json for scenario_18")?;
    assert_eq!(fixture.schema_version, 1, "fixture schema_version drifted");

    let config = fixture.workspace_files.iter().fold(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() },
        |cfg, f| cfg.with_file(&f.path, &f.content),
    );

    let harness = UxHarness::new(config).context("create UX harness")?;
    for file in &fixture.workspace_files {
        harness
            .open_file(&file.path, &file.content)
            .with_context(|| format!("didOpen {}", file.path))?;
    }

    std::thread::sleep(Duration::from_millis(700));

    let mut checks_run = 0_u32;
    let mut checks_passed = 0_u32;

    // hover
    checks_run += 1;
    let hover = harness.hover(
        &fixture.checks.hover.position.path,
        fixture.checks.hover.position.line,
        fixture.checks.hover.position.character,
    )?;
    if hover
        .as_ref()
        .map(|value| value.to_string().contains(&fixture.checks.hover.contains))
        .unwrap_or(false)
    {
        checks_passed += 1;
    } else {
        anyhow::bail!("hover expectation failed: expected payload to contain marker");
    }

    // completion
    checks_run += 1;
    let completion = harness.completion(
        &fixture.checks.completion.position.path,
        fixture.checks.completion.position.line,
        fixture.checks.completion.position.character,
    )?;
    let completion_labels = completion
        .iter()
        .filter_map(|item| item.get("label").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if fixture
        .checks
        .completion
        .must_include
        .iter()
        .all(|needle| completion_labels.iter().any(|label| label == needle))
    {
        checks_passed += 1;
    } else {
        anyhow::bail!("completion expectation failed: required labels missing");
    }

    // definition + declaration
    checks_run += 2;
    let definition = harness.definition(
        &fixture.checks.definition.position.path,
        fixture.checks.definition.position.line,
        fixture.checks.definition.position.character,
    )?;
    assert!(
        definition.len() >= fixture.checks.definition.min_results,
        "definition expected at least {} results, got {}",
        fixture.checks.definition.min_results,
        definition.len()
    );
    assert!(
        definition.iter().any(|loc| {
            loc.get("uri")
                .and_then(Value::as_str)
                .map(|uri| uri.ends_with(&fixture.checks.definition.target_suffix))
                .unwrap_or(false)
        }),
        "definition should target {}",
        fixture.checks.definition.target_suffix
    );
    checks_passed += 1;

    let declaration = harness.declaration(
        &fixture.checks.declaration.position.path,
        fixture.checks.declaration.position.line,
        fixture.checks.declaration.position.character,
    )?;
    assert!(
        declaration.len() >= fixture.checks.declaration.min_results,
        "declaration expected at least {} results, got {}",
        fixture.checks.declaration.min_results,
        declaration.len()
    );
    assert!(
        declaration.iter().any(|loc| {
            loc.get("uri")
                .and_then(Value::as_str)
                .map(|uri| uri.ends_with(&fixture.checks.declaration.target_suffix))
                .unwrap_or(false)
        }),
        "declaration should target {}",
        fixture.checks.declaration.target_suffix
    );
    checks_passed += 1;

    // references
    checks_run += 1;
    let references = harness.references(
        &fixture.checks.references.position.path,
        fixture.checks.references.position.line,
        fixture.checks.references.position.character,
        fixture.checks.references.include_declaration,
    )?;
    assert!(
        references.len() >= fixture.checks.references.min_results,
        "references expected at least {} results, got {}",
        fixture.checks.references.min_results,
        references.len()
    );
    checks_passed += 1;

    // document symbols + workspace symbols
    checks_run += 2;
    let doc_symbols = harness.document_symbols(&fixture.checks.document_symbols.path)?;
    let symbol_blob = doc_symbols.iter().map(Value::to_string).collect::<String>();
    assert!(
        fixture
            .checks
            .document_symbols
            .must_include
            .iter()
            .all(|needle| symbol_blob.contains(needle)),
        "document symbols must include expected names"
    );
    checks_passed += 1;

    let workspace_symbols = harness.workspace_symbols(&fixture.checks.workspace_symbols.query)?;
    assert!(
        workspace_symbols.len() >= fixture.checks.workspace_symbols.min_results,
        "workspace/symbol expected at least {} results, got {}",
        fixture.checks.workspace_symbols.min_results,
        workspace_symbols.len()
    );
    checks_passed += 1;

    // rename
    checks_run += 1;
    let rename = harness.rename(
        &fixture.checks.rename.position.path,
        fixture.checks.rename.position.line,
        fixture.checks.rename.position.character,
        &fixture.checks.rename.new_name,
    )?;
    let rename_edit_count = rename
        .as_ref()
        .and_then(|payload| payload.get("changes"))
        .and_then(Value::as_object)
        .map(|changes| {
            changes.values().filter_map(Value::as_array).map(std::vec::Vec::len).sum::<usize>()
        })
        .unwrap_or(0);
    assert!(
        rename_edit_count >= fixture.checks.rename.min_edits,
        "rename expected at least {} edits, got {}",
        fixture.checks.rename.min_edits,
        rename_edit_count
    );
    checks_passed += 1;

    // diagnostics after open / after edit
    checks_run += 1;
    let diags_after_open =
        harness.wait_for_diagnostics(&fixture.checks.diagnostics.path, Duration::from_secs(2));
    assert!(
        diags_after_open.len() >= fixture.checks.diagnostics.after_open_min,
        "diagnostics expected at least {} after open, got {}",
        fixture.checks.diagnostics.after_open_min,
        diags_after_open.len()
    );

    harness.change_file_full(
        &fixture.checks.diagnostics.path,
        &fixture.checks.diagnostics.fixed_content,
        2,
    )?;
    let diags_after_fix =
        harness.wait_for_diagnostics(&fixture.checks.diagnostics.path, Duration::from_secs(2));
    assert!(
        diags_after_fix.len() <= fixture.checks.diagnostics.after_fix_max,
        "diagnostics expected at most {} after edit, got {}",
        fixture.checks.diagnostics.after_fix_max,
        diags_after_fix.len()
    );
    checks_passed += 1;

    harness.assert_no_crash();

    let pass_rate = f64::from(checks_passed) / f64::from(checks_run);
    eprintln!(
        "scenario_18 scorecard: checks_passed={checks_passed} checks_run={checks_run} pass_rate={pass_rate:.2}"
    );

    Ok(())
}
