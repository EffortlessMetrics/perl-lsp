//! LSP 3.17 Document Feature Contract Tests
//!
//! Tests for codeLens, documentLink, documentColor, foldingRange,
//! selectionRange, and linkedEditingRange.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== CODE LENS ====================

#[test]
fn test_code_lens_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "sub test {}\ntest();")?;

    let response = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

// ==================== DOCUMENT LINK ====================

#[test]
fn test_document_link_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "use strict;\nuse Data::Dumper;")?;

    let response = harness.request(
        "textDocument/documentLink",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

// ==================== DOCUMENT COLOR ====================

#[test]
fn test_document_color_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.css", ".class { color: #FF0000; }")?;

    // May not be supported for Perl
    let response = harness.request(
        "textDocument/documentColor",
        json!({
            "textDocument": { "uri": "file:///test.css" }
        }),
    );

    if let Ok(colors) = response {
        assert!(colors.is_null() || colors.is_array());
    }
    Ok(())
}

// ==================== FOLDING RANGE ====================

#[test]
fn test_folding_range_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open(
        "file:///test.pl",
        "sub test {\n    my $x = 1;\n    return $x;\n}",
    )?;

    let response = harness.request(
        "textDocument/foldingRange",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

// ==================== SELECTION RANGE ====================

#[test]
fn test_selection_range_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "if ($x) { print $x; }")?;

    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "positions": [
                { "line": 0, "character": 10 }
            ]
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

// ==================== LINKED EDITING RANGE ====================

#[test]
fn test_linked_editing_range_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "<div></div>")?;

    let response = harness.request(
        "textDocument/linkedEditingRange",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 1 }
        }),
    );

    if let Ok(ranges) = response {
        assert!(ranges.is_null() || (ranges.is_object() && ranges["ranges"].is_array()));
    }
    Ok(())
}
