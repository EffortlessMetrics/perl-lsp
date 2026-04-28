//! LSP 3.17 Diagnostics, Inlay Hints, Inline Values, and Moniker Contract Tests
//!
//! Tests for textDocument/diagnostic, workspace/diagnostic, textDocument/inlayHint,
//! textDocument/inlineValue, and textDocument/moniker.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== DIAGNOSTICS PULL MODEL (3.17) ====================

#[test]
fn test_diagnostic_pull_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "$undefined")?;

    let response = harness.request(
        "textDocument/diagnostic",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "identifier": "perl-lsp",
            "previousResultId": null
        }),
    );

    if let Ok(report) = response {
        if !report.is_null() {
            assert!(report["kind"].is_string());
            if report["kind"] == "full" {
                assert!(report["items"].is_array());
            }
        }
    }
    Ok(())
}

#[test]
fn test_workspace_diagnostic_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let response = harness.request(
        "workspace/diagnostic",
        json!({
            "identifier": "perl-lsp",
            "previousResultIds": [],
            "workDoneToken": "diag-1",
            "partialResultToken": "partial-1"
        }),
    );

    if let Ok(report) = response {
        assert!(report.is_null() || report.is_object());
    }
    Ok(())
}

// ==================== INLAY HINTS (3.17) ====================

#[test]
fn test_inlay_hint_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "substr($str, 0, 5)")?;

    let response = harness.request(
        "textDocument/inlayHint",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 18 }
            }
        }),
    );

    if let Ok(hints) = response {
        assert!(hints.is_null() || hints.is_array());
    }
    Ok(())
}

// ==================== INLINE VALUES (3.17) ====================

#[test]
fn test_inline_value_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $x = 42;\nprint $x;")?;

    let response = harness.request(
        "textDocument/inlineValue",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 1, "character": 9 }
            },
            "context": {
                "frameId": 1,
                "stoppedLocation": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 1, "character": 9 }
                }
            }
        }),
    );

    if let Ok(values) = response {
        assert!(values.is_null() || values.is_array());
    }
    Ok(())
}

// ==================== MONIKER (3.16+) ====================

#[test]
fn test_moniker_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "package Foo::Bar;\nsub test {}")?;

    let response = harness.request(
        "textDocument/moniker",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 4 }
        }),
    );

    if let Ok(monikers) = response {
        assert!(monikers.is_null() || monikers.is_array());
    }
    Ok(())
}
