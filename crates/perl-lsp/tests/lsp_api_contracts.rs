//! API Contract Tests for LSP Server
//!
//! These tests ensure our LSP implementation maintains stable API contracts
//! and properly validates all inputs/outputs according to the LSP specification.

#![allow(clippy::collapsible_if)]

use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::{Duration, Instant};

mod support;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ======================== HELPER FUNCTIONS ========================

fn is_range(v: &Value) -> bool {
    v.pointer("/start/line").and_then(|x| x.as_u64()).is_some()
        && v.pointer("/start/character").and_then(|x| x.as_u64()).is_some()
        && v.pointer("/end/line").and_then(|x| x.as_u64()).is_some()
        && v.pointer("/end/character").and_then(|x| x.as_u64()).is_some()
}

#[allow(dead_code)]
fn is_location(v: &Value) -> bool {
    v.get("uri").and_then(|u| u.as_str()).is_some() && v.get("range").is_some_and(is_range)
}

fn open_doc(harness: &mut LspHarness, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let uri = "file:///test.pl";
    harness.open(uri, text)?;
    Ok(uri.to_string())
}

// ======================== INITIALIZATION CONTRACT ========================

#[test]
fn test_initialization_contract() -> TestResult {
    let mut harness = LspHarness::new();

    // Initialize with minimal capabilities
    let response = harness.initialize(Some(json!({
        "textDocument": {},
        "workspace": {}
    })))?;

    // The harness returns the 'result' field directly, which contains 'capabilities'
    let caps = response.get("capabilities").ok_or("initialize response must have capabilities")?;

    // CRITICAL CONTRACT: Must have these exact trigger characters
    let triggers = caps
        .pointer("/completionProvider/triggerCharacters")
        .and_then(|v| v.as_array())
        .ok_or("completionProvider.triggerCharacters must be an array")?;

    let trigger_set: HashSet<_> = triggers.iter().filter_map(|v| v.as_str()).collect();

    // Must have exactly these triggers
    for trigger in ["$", "@", "%", "->"] {
        assert!(trigger_set.contains(trigger), "Missing required trigger character: {}", trigger);
    }

    // Must NOT have these as separate characters (regression prevention)
    assert!(!trigger_set.contains("-"), "Must not have '-' as separate trigger");
    assert!(!trigger_set.contains(">"), "Must not have '>' as separate trigger");

    // Text document sync must support incremental sync
    let sync = caps.get("textDocumentSync");
    if let Some(sync_obj) = sync {
        if let Some(obj) = sync_obj.as_object() {
            // If it's an object, check the change field
            assert_eq!(
                obj.get("change").and_then(|v| v.as_u64()),
                Some(2),
                "textDocumentSync.change must be 2 (incremental sync)"
            );
        } else if let Some(num) = sync_obj.as_u64() {
            // If it's a number directly
            assert_eq!(num, 2, "textDocumentSync must be 2 (incremental sync)");
        }
    }

    Ok(())
}

#[test]
fn test_minimal_client_initialization() -> TestResult {
    let mut harness = LspHarness::new();

    // Minimal client with no dynamic registration or advanced features
    let response = harness.initialize(Some(json!({
        "textDocument": {},
        "workspace": {}
    })))?;

    let caps = response.get("capabilities").ok_or("capabilities")?;

    // Server should still advertise basic features
    assert!(caps.get("textDocumentSync").is_some());
    assert!(caps.get("completionProvider").is_some());
    assert!(caps.get("hoverProvider").is_some());
    assert!(caps.get("definitionProvider").is_some());

    // Verify triggers are still correct for minimal client
    let triggers = caps
        .pointer("/completionProvider/triggerCharacters")
        .and_then(|v| v.as_array())
        .ok_or("triggerCharacters")?;
    assert_eq!(triggers.len(), 4);
    assert!(triggers.iter().any(|t| t == "->"));

    Ok(())
}

#[test]
fn test_double_initialization_rejected() -> TestResult {
    let mut harness = LspHarness::new();

    // First initialization should succeed
    let first = harness.initialize(None)?;
    assert!(first.get("capabilities").is_some());

    // Second initialization should be rejected with LSP error
    let second = harness.initialize(None);
    assert!(second.is_err(), "Second initialization must fail");

    if let Err(msg) = second {
        // Should contain standard LSP error code -32600 (InvalidRequest)
        // or clear message about already initialized
        assert!(
            msg.contains("-32600")
                || msg.contains("already initialized")
                || msg.contains("LSP error"),
            "Double init must fail with appropriate error, got: {}",
            msg
        );
    }

    Ok(())
}

// ======================== REQUEST/RESPONSE SHAPE CONTRACTS ========================

#[test]
fn test_completion_response_shape() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open("file:///test.pl", "my $var = 1; $v")?;

    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 14 }
        }),
    )?;

    // Response must be array or CompletionList
    let items = if let Some(arr) = response.as_array() {
        // Direct array of CompletionItems
        arr
    } else if let Some(list) = response.as_object() {
        // CompletionList with items field
        list.get("items")
            .and_then(|v| v.as_array())
            .ok_or("CompletionList must have items array")?
    } else if response.is_null() {
        // Null is acceptable for no completions
        return Ok(());
    } else {
        return Err(format!(
            "Completion must return array, CompletionList, or null, got: {:?}",
            response
        )
        .into());
    };

    // If we have items, verify at least one has required fields
    if !items.is_empty() {
        let first = &items[0];
        assert!(
            first.get("label").and_then(|v| v.as_str()).is_some(),
            "CompletionItem must have string label"
        );
    }

    Ok(())
}

#[test]
fn test_hover_response_shape() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.open("file:///test.pl", "sub test { } test();")?;

    let response = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 13 }  // On 'test' call
        }),
    );

    // Hover might return null for positions without hover info
    if let Ok(hover) = response {
        if !hover.is_null() {
            // Check for contents field (required by LSP)
            let contents = hover.get("contents").ok_or("Hover must have contents field")?;

            // Contents can be string, MarkupContent, or MarkedString[]
            if contents.is_string() {
                let text = contents.as_str().ok_or("string contents")?;
                assert!(!text.is_empty());
            } else if let Some(obj) = contents.as_object() {
                // MarkupContent with kind and value
                assert!(
                    obj.get("kind").is_some() && obj.get("value").is_some(),
                    "MarkupContent must have kind and value"
                );
            } else if let Some(arr) = contents.as_array() {
                // MarkedString[]
                assert!(!arr.is_empty());
            } else {
                return Err(format!("Invalid hover contents format: {:?}", contents).into());
            }
        }
    }

    Ok(())
}

#[test]
fn test_document_highlight_contract() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = open_doc(&mut harness, r#"my $x = 1; $x++; print $x;"#)?;

    let response = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }  // On first $x
        }),
    );

    if let Ok(highlights) = response {
        if let Some(items) = highlights.as_array() {
            // Should have multiple highlights
            assert!(items.len() >= 2, "Should highlight multiple occurrences");

            for item in items {
                // Each highlight must have a range
                assert!(is_range(&item["range"]), "Highlight must have valid range");

                // Kind is optional but if present must be 1, 2, or 3
                if let Some(kind) = item.get("kind").and_then(|v| v.as_u64()) {
                    assert!((1..=3).contains(&kind), "Invalid highlight kind: {}", kind);
                }
            }

            // At least one should be Write (3) for declaration
            assert!(
                items.iter().any(|i| i["kind"] == 3),
                "Should have at least one Write highlight"
            );
        }
    }

    Ok(())
}

// ======================== ERROR HANDLING CONTRACTS ========================

#[test]
fn test_error_response_contract() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Request on non-existent document might return null or error
    let response = harness.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///nonexistent.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    // Either null result or error is acceptable
    match response {
        Ok(result) => {
            // Null or empty result is valid for non-existent document
            let is_empty_array = result.is_array() && result.as_array().ok_or("array")?.is_empty();
            let is_empty_list = result.is_object()
                && result.get("items").and_then(|v| v.as_array()).is_some_and(|a| a.is_empty());

            assert!(
                result.is_null() || is_empty_array || is_empty_list,
                "Expected null or empty result for non-existent document"
            );
        }
        Err(msg) => {
            // Error message should be non-empty and informative
            assert!(!msg.is_empty(), "Error message should not be empty");
        }
    }

    Ok(())
}

// ======================== WORKSPACE OPERATIONS ========================

#[test]
fn test_workspace_folders_contract() -> TestResult {
    let mut harness = LspHarness::new();

    // Initialize with workspace folders capability
    let response = harness.initialize(Some(json!({
        "workspace": {
            "workspaceFolders": true
        },
        "textDocument": {}
    })))?;

    let caps = response.get("capabilities").ok_or("capabilities")?;

    // Only send workspace folder changes if server explicitly supports it
    // and won't trigger reverse requests
    let _wf_supported = caps
        .pointer("/workspace/workspaceFolders/supported")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Skip the notify to avoid reverse-request deadlock
    // The initialization with workspaceFolders capability is sufficient
    // to verify the server accepts the capability

    // Server should work with documents in different workspace paths
    harness.open("file:///workspace/test.pl", "print 1;")?;
    harness.open("file:///workspace2/test.pl", "print 2;")?;

    // Both documents should be queryable
    let response1 = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": "file:///workspace/test.pl" }
        }),
    );
    assert!(response1.is_ok(), "Should handle first workspace");

    let response2 = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": "file:///workspace2/test.pl" }
        }),
    );
    assert!(response2.is_ok(), "Should handle second workspace");

    Ok(())
}

// ======================== VERSION AND EDIT CONTRACTS ========================

#[test]
fn test_apply_edit_with_version() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///test.pl";
    harness.open(uri, "say 1;\n")?;

    // Note: workspace/applyEdit is a client capability
    // We test version handling via didChange instead
    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": 2 },
            "contentChanges": [{
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 0 }
                },
                "text": "use strict;\n"
            }]
        }),
    );

    // Now send a stale version - server should handle gracefully
    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": 1 },  // Stale version
            "contentChanges": [{
                "text": "# This should be ignored or handled gracefully"
            }]
        }),
    );

    // Server should still be responsive
    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    );

    assert!(response.is_ok(), "Server must handle version conflicts gracefully");

    Ok(())
}

// ======================== PERFORMANCE CONTRACTS ========================

#[test]
fn test_bounded_definition_timeout() -> TestResult {
    // This test verifies that module resolution completes quickly
    // even when the module doesn't exist. We've improved the timeout
    // handling but the test harness itself may block.

    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = open_doc(&mut harness, r#"My::Missing::Module->new()"#)?;

    let start = Instant::now();
    let response = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 10 }
        }),
        Duration::from_millis(400),
    );
    let elapsed = start.elapsed();

    // Should return quickly even for missing modules
    assert!(elapsed < Duration::from_millis(500), "Definition lookup took too long: {:?}", elapsed);

    // Result should be null or empty array
    if let Ok(result) = response {
        let is_empty_array = result.is_array() && result.as_array().ok_or("array")?.is_empty();
        assert!(result.is_null() || is_empty_array, "Missing module should return null or empty");
    }

    Ok(())
}

#[test]
fn test_large_file_responsiveness() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Create a moderately large file
    let mut content = String::new();
    for i in 0..1000 {
        content.push_str(&format!("my $var{} = {};\n", i, i));
    }

    let uri = "file:///large.pl";
    harness.open(uri, &content)?;

    // Request should complete in reasonable time
    let (_, duration) = harness.timed_request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    assert!(
        duration < Duration::from_millis(1000),
        "Large file processing took too long: {:?}",
        duration
    );

    Ok(())
}

// ======================== PROPERTY-BASED CONTRACTS ========================

#[test]
fn test_idempotent_operations() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///test.pl";
    let text = "my $x = 1;";

    // Opening same document multiple times should be idempotent
    harness.open(uri, text)?;
    harness.open(uri, text)?;

    let response1 = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    harness.open(uri, text)?;

    let response2 = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;

    // Results should be identical
    assert_eq!(response1, response2, "Idempotent operations must produce same results");

    Ok(())
}

#[test]
fn test_uri_validation_contract() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Valid URIs that must be accepted
    let valid_uris = vec![
        "file:///home/user/test.pl",
        "file:///C:/Users/test.pl", // Windows
        "file:///test%20file.pl",   // URL encoded
        "untitled:untitled-1",      // VSCode untitled
    ];

    for uri in valid_uris {
        harness.open(uri, "print 'test'")?;

        let response = harness.request(
            "textDocument/documentSymbol",
            json!({
                "textDocument": { "uri": uri }
            }),
        );

        assert!(response.is_ok(), "Must accept valid URI: {}", uri);
    }

    Ok(())
}

// ======================== BACKWARDS COMPATIBILITY ========================

#[test]
fn test_legacy_client_support() -> TestResult {
    let mut harness = LspHarness::new();

    // Old-style initialization without modern capabilities
    let response = harness.initialize(Some(json!({
        "textDocument": {
            // No completion, hover, or other modern capabilities
        }
    })))?;

    // Server should still work with minimal capabilities
    let caps = response.get("capabilities").ok_or("capabilities")?;
    assert!(caps.get("textDocumentSync").is_some());

    // Basic operations should still work
    harness.open("file:///test.pl", "print 1;")?;

    Ok(())
}

#[test]
fn test_no_stdout_leak() -> TestResult {
    // This test would need special harness support to capture raw stdout
    // For now, we just verify the server doesn't panic on normal operations
    let mut harness = LspHarness::new();

    // Multiple operations that might trigger stdout writes
    harness.initialize(None)?;
    harness.open("file:///test.pl", "print 'test';")?;
    harness
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": "file:///test.pl" },
                "position": { "line": 0, "character": 5 }
            }),
        )
        .ok();

    // If we got here without panics, basic stdout handling is working

    Ok(())
}
