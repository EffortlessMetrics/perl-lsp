//! LSP 3.17 Rename Contract Tests
//!
//! Tests for textDocument/rename and textDocument/prepareRename.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== RENAME ====================

#[test]
fn test_rename_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $old = 1;\n$old++;")?;

    let response = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 },
            "newName": "new"
        }),
    )?;

    assert!(response.is_null() || response.is_object());
    Ok(())
}

#[test]
fn test_prepare_rename_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $var = 1;")?;

    let response = harness.request(
        "textDocument/prepareRename",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 }
        }),
    )?;

    // Can be Range, {range, placeholder}, {defaultBehavior}, or null
    assert!(response.is_null() || response.is_object() || response.is_array());
    Ok(())
}
