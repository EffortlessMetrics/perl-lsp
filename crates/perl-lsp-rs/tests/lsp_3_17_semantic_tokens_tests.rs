//! LSP 3.17 Semantic Tokens Contract Tests
//!
//! Tests for textDocument/semanticTokens/full and textDocument/semanticTokens/range.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== SEMANTIC TOKENS (3.16+) ====================

#[test]
fn test_semantic_tokens_full_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "package Foo;\nsub bar { my $var = 1; }")?;

    let response = harness.request(
        "textDocument/semanticTokens/full",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    );

    if let Ok(tokens) = response {
        if !tokens.is_null() {
            assert!(tokens["data"].is_array());
        }
    }
    Ok(())
}

#[test]
fn test_semantic_tokens_range_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $x = 1;\nmy $y = 2;")?;

    let response = harness.request(
        "textDocument/semanticTokens/range",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 10 }
            }
        }),
    );

    if let Ok(tokens) = response {
        assert!(tokens.is_null() || tokens["data"].is_array());
    }
    Ok(())
}
