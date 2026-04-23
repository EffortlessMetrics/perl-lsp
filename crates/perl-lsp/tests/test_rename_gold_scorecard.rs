//! Editor Intelligence Scorecard — Rename Gold Corpus Tests (RED)
//!
//! These tests define what the rename scorecard must look like.
//! They FAIL until the implementation provides:
//!   - `RenameGoldFixture` struct in `perl-corpus/src/gold.rs`
//!   - `RenameAssertionKind` enum with assertion variants
//!   - `load_rename_gold_fixtures()` function
//!   - Gold fixtures in `test_corpus/gold/<name>/expected_rename.json`
//!
//! ## What Correct Behavior Looks Like
//!
//! 1. `RenameNonNull` — rename returns non-null WorkspaceEdit for renamable symbols
//! 2. `RenamePrepareValid` — prepareRename returns a valid range at renamable positions
//! 3. `RenamePrepareNull` — prepareRename returns null at non-renamable positions (keywords, comments)
//! 4. `RenameChangesContainText` — WorkspaceEdit changes contain the new name text
//! 5. `RenameChangesMatchCount` — rename touches the expected number of locations

mod common;

use common::test_utils::TestServerBuilder;
use perl_corpus::gold::{RenameAssertionKind, RenameGoldFixture, load_rename_gold_fixtures};
use serde_json::Value;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn gold_corpus_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let crate_dir = PathBuf::from(manifest);
    let workspace_root = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| crate_dir.clone());
    workspace_root.join("test_corpus").join("gold")
}

fn is_workspace_edit_non_null(resp: &Value) -> bool {
    resp.get("result").map(|r| !r.is_null() && r.is_object()).unwrap_or(false)
}

fn workspace_edit_change_count(resp: &Value) -> usize {
    resp.get("result")
        .and_then(|r| r.get("changes"))
        .and_then(|c| c.as_object())
        .map(|obj| obj.len())
        .unwrap_or(0)
}

fn workspace_edit_contains_text(resp: &Value, text: &str) -> bool {
    fn search_for_text(value: &serde_json::Value, text: &str) -> bool {
        match value {
            serde_json::Value::String(s) => s.contains(text),
            serde_json::Value::Array(arr) => arr.iter().any(|v| search_for_text(v, text)),
            serde_json::Value::Object(obj) => obj.values().any(|v| search_for_text(v, text)),
            _ => false,
        }
    }
    resp.get("result").is_some_and(|r| search_for_text(r, text))
}

fn prepare_rename_response(resp: &Value) -> Option<Value> {
    resp.get("result").cloned().filter(|r| !r.is_null())
}

// ---------------------------------------------------------------------------
// Rename Gold Corpus Test
// ---------------------------------------------------------------------------

/// Run all rename gold fixtures and assert every assertion passes.
///
/// This test will FAIL until `RenameGoldFixture`, `RenameAssertionKind`,
/// and `load_rename_gold_fixtures()` exist in `perl-corpus/src/gold.rs`.
#[test]
fn test_rename_gold_corpus() {
    let root = gold_corpus_root();
    let fixtures: Vec<RenameGoldFixture> = match load_rename_gold_fixtures(&root) {
        Ok(f) => {
            // RED TEST: fixtures MUST exist for a valid scorecard
            // If this assertion fails, it means code-builder has not yet
            // created gold fixtures for rename testing.
            assert!(
                !f.is_empty(),
                "RED TEST FAILURE: No rename gold fixtures found in {}. \
                Code-builder must create expected_rename.json fixtures in test_corpus/gold/<name>/.",
                root.display()
            );
            f
        }
        Err(e) => panic!("Failed to load rename gold fixtures: {e}"),
    };

    let server = TestServerBuilder::new().build();

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for fixture in &fixtures {
        let code = std::fs::read_to_string(&fixture.fixture_path).unwrap_or_else(|e| {
            panic!("Cannot read fixture {}: {e}", fixture.fixture_path.display())
        });

        let uri = format!("file:///gold/{}.pl", fixture.name);
        server.open_document(&uri, &code);

        for assertion in &fixture.rename_assertions {
            total += 1;

            // First call prepareRename to check if the position is renamable
            let prepare_resp = server.prepare_rename(&uri, assertion.line, assertion.character);

            // Then call rename with the new name
            let rename_resp =
                server.rename(&uri, assertion.line, assertion.character, &assertion.new_name);

            let ok = match &assertion.kind {
                RenameAssertionKind::RenameNonNull => is_workspace_edit_non_null(&rename_resp),
                RenameAssertionKind::RenameNull => {
                    // null result is expected for non-renamable positions
                    rename_resp.get("result").map(|r| r.is_null()).unwrap_or(true)
                }
                RenameAssertionKind::RenamePrepareValid => {
                    // prepareRename should return a valid range for renamable symbols
                    prepare_rename_response(&prepare_resp).is_some()
                }
                RenameAssertionKind::RenamePrepareNull => {
                    // prepareRename should return null for non-renamable positions
                    prepare_rename_response(&prepare_resp).is_none()
                }
                RenameAssertionKind::RenameChangesContainText { expected_text } => {
                    workspace_edit_contains_text(&rename_resp, expected_text)
                }
                RenameAssertionKind::RenameChangesMatchCount { expected_count } => {
                    let actual = workspace_edit_change_count(&rename_resp);
                    actual == *expected_count
                }
            };

            if ok {
                passed += 1;
            } else {
                failures.push(format!(
                    "  FAIL [{}] {:?} at line:{} char:{} new_name:{} — prepare: {:?} rename: {:?}",
                    fixture.name,
                    assertion.kind,
                    assertion.line,
                    assertion.character,
                    assertion.new_name,
                    prepare_resp.get("result").map(|r| r.to_string()).unwrap_or_default(),
                    rename_resp.get("result").map(|r| r.to_string()).unwrap_or_default(),
                ));
            }
        }
    }

    println!(
        "\nRename gold corpus: {}/{} assertions passed ({:.0}%)",
        passed,
        total,
        if total > 0 { passed as f64 / total as f64 * 100.0 } else { 100.0 }
    );
    for f in &failures {
        println!("{f}");
    }

    assert!(
        failures.is_empty(),
        "Rename gold corpus: {} assertion(s) failed out of {}:\n{}",
        failures.len(),
        total,
        failures.join("\n")
    );
}
