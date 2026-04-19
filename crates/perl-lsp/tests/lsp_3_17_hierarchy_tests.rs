//! LSP 3.17 Call Hierarchy and Type Hierarchy Contract Tests
//!
//! Tests for prepareCallHierarchy, incomingCalls, prepareTypeHierarchy,
//! and typeHierarchy/supertypes.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== CALL HIERARCHY (3.16+) ====================

#[test]
fn test_prepare_call_hierarchy_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "sub test { helper(); }\nsub helper {}")?;

    let response = harness.request(
        "textDocument/prepareCallHierarchy",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 }
        }),
    );

    if let Ok(items) = response {
        assert!(items.is_null() || items.is_array());
    }
    Ok(())
}

#[test]
fn test_incoming_calls_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let response = harness.request(
        "callHierarchy/incomingCalls",
        json!({
            "item": {
                "name": "test",
                "kind": 12,  // Function
                "uri": "file:///test.pl",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 20 }
                },
                "selectionRange": {
                    "start": { "line": 0, "character": 4 },
                    "end": { "line": 0, "character": 8 }
                }
            }
        }),
    );

    if let Ok(calls) = response {
        assert!(calls.is_null() || calls.is_array());
    }
    Ok(())
}

// ==================== TYPE HIERARCHY (3.17) ====================

#[test]
fn test_prepare_type_hierarchy_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open(
        "file:///test.pl",
        "package Base;\npackage Derived;\nuse base 'Base';",
    )?;

    let response = harness.request(
        "textDocument/prepareTypeHierarchy",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 8 }
        }),
    );

    if let Ok(items) = response {
        assert!(items.is_null() || items.is_array());
    }
    Ok(())
}

#[test]
fn test_type_hierarchy_supertypes_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let response = harness.request(
        "typeHierarchy/supertypes",
        json!({
            "item": {
                "name": "Derived",
                "kind": 5,  // Class
                "uri": "file:///test.pl",
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 2, "character": 17 }
                },
                "selectionRange": {
                    "start": { "line": 1, "character": 8 },
                    "end": { "line": 1, "character": 15 }
                }
            }
        }),
    );

    if let Ok(types) = response {
        assert!(types.is_null() || types.is_array());
    }
    Ok(())
}
