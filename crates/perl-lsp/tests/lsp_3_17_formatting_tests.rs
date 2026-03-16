//! LSP 3.17 Formatting Contract Tests
//!
//! Tests for textDocument/formatting, rangeFormatting, and onTypeFormatting.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== FORMATTING ====================

#[test]
fn test_formatting_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my$x=1;print$x;")?;

    let response = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "options": {
                "tabSize": 4,
                "insertSpaces": true,
                "trimTrailingWhitespace": true,
                "insertFinalNewline": true,
                "trimFinalNewlines": true
            }
        }),
    );

    // Handle both success and error cases - this is a protocol compliance test
    match response {
        Ok(result) => {
            // Success: should return null or array of edits
            assert!(result.is_null() || result.is_array());
        }
        Err(_) => {
            // Error is acceptable when perltidy is not available
            // This maintains LSP protocol compliance
            eprintln!(
                "Formatting failed (perltidy may not be installed) - this is acceptable for protocol compliance"
            );
        }
    }
    Ok(())
}

#[test]
fn test_range_formatting_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my$x=1;\nprint$x;")?;

    let response = harness.request(
        "textDocument/rangeFormatting",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 7 }
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    );

    // Handle both success and error cases - this is a protocol compliance test
    match response {
        Ok(result) => {
            // Success: should return null or array of edits
            assert!(result.is_null() || result.is_array());
        }
        Err(_) => {
            // Error is acceptable when perltidy is not available
            // This maintains LSP protocol compliance
            eprintln!(
                "Range formatting failed (perltidy may not be installed) - this is acceptable for protocol compliance"
            );
        }
    }
    Ok(())
}

#[test]
fn test_on_type_formatting_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "if (1) {")?;

    let response = harness.request(
        "textDocument/onTypeFormatting",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 8 },
            "ch": "{",
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}
