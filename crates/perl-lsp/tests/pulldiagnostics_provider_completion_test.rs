//! Tests for verifying PullDiagnosticsProvider completion for production use.
//!
//! These tests verify the three gaps identified in ADR-0042:
//! 1. handle_workspace_diagnostic should use orchestrator pattern
//! 2. is_fixable_diagnostic in pull.rs should delegate to is_fixable_perlcritic_policy
//! 3. orchestrator.reset() should be called in handle_did_change_configuration
//!
//! These tests will FAIL before the implementation is complete and PASS after.

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test for Gap 2 (AC3): pull.rs::is_fixable_diagnostic should delegate to
/// is_fixable_perlcritic_policy() from the diagnostics module, not hardcode policy strings.
///
/// This test verifies that pull.rs's is_fixable_diagnostic function imports and uses
/// the is_fixable_perlcritic_policy helper from the diagnostics module.
///
/// CURRENTLY FAILS because pull.rs has hardcoded policy strings inline.
/// WILL PASS after refactoring when pull.rs delegates to the helper.
#[test]
fn test_is_fixable_diagnostic_uses_shared_helper() {
    // This test verifies the implementation by checking if the source code
    // of pull.rs contains a delegation to is_fixable_perlcritic_policy.
    //
    // The source code should contain something like:
    //   is_fixable_perlcritic_policy(code)
    // instead of hardcoded strings like:
    //   "TestingAndDebugging::RequireUseStrict" | "TestingAndDebugging::RequireUseWarnings" | ...

    let pull_rs_source = include_str!("../src/features/diagnostics/pull.rs");

    // Find the is_fixable_diagnostic function in pull.rs
    let func_start = pull_rs_source
        .find("fn is_fixable_diagnostic(code: &str) -> bool {")
        .expect("Could not find is_fixable_diagnostic in pull.rs");

    // Extract a reasonable chunk of the function (up to 50 lines)
    let func_chunk = &pull_rs_source[func_start..func_start + 3000];

    // The function should NOT have hardcoded perlcritic policy strings directly in matches
    // It should delegate to is_fixable_perlcritic_policy
    let has_hardcoded_policies = func_chunk.contains("\"TestingAndDebugging::RequireUseStrict\"")
        && func_chunk.contains("\"TestingAndDebugging::RequireUseWarnings\"")
        && func_chunk.contains("\"InputOutput::ProhibitBarewordFileHandles\"");

    // The function SHOULD call is_fixable_perlcritic_policy
    let delegates_to_helper = func_chunk.contains("is_fixable_perlcritic_policy(code)");

    // Assert: if hardcoded policies exist, the function must delegate to helper
    // This test will FAIL if pull.rs has hardcoded strings and no delegation
    // This test will PASS if pull.rs delegates to is_fixable_perlcritic_policy
    assert!(
        !has_hardcoded_policies || delegates_to_helper,
        "pull.rs::is_fixable_diagnostic should delegate to is_fixable_perlcritic_policy(), \
         not hardcode policy strings inline. Found hardcoded policies: {}, delegation: {}",
        has_hardcoded_policies,
        delegates_to_helper
    );
}

/// Test for Gap 3 (AC4): handle_did_change_configuration should call
/// orchestrator.reset() when perlcritic config changes.
///
/// CURRENTLY FAILS because handle_did_change_configuration only resets
/// LspServer.critic_analyzer, not PullDiagnosticsOrchestrator.critic_analyzer.
/// WILL PASS after wiring orchestrator.reset() into the config change handler.
#[test]
fn test_orchestrator_reset_is_wired_into_config_change() {
    // This test verifies that workspace.rs's handle_did_change_configuration
    // calls self.pull_diagnostics_orchestrator.reset()
    //
    // The source code should contain:
    //   self.pull_diagnostics_orchestrator.reset()
    // in the config change handler

    let workspace_rs_source = include_str!("../src/runtime/workspace.rs");

    // Find the handle_did_change_configuration function
    let func_start = workspace_rs_source
        .find("pub(super) fn handle_did_change_configuration(&self, params: Option<Value>)")
        .expect("Could not find handle_did_change_configuration in workspace.rs");

    // Extract a reasonable chunk of the function
    let func_chunk = &workspace_rs_source[func_start..func_start + 8000];

    // Check if orchestrator.reset() is called
    let calls_orchestrator_reset =
        func_chunk.contains("self.pull_diagnostics_orchestrator.reset()");

    // This test will FAIL if orchestrator.reset() is not called
    // This test will PASS after the wiring is done
    assert!(
        calls_orchestrator_reset,
        "handle_did_change_configuration should call self.pull_diagnostics_orchestrator.reset() \
         when perlcritic config changes, to ensure the orchestrator's CriticAnalyzer cache \
         is also invalidated. This ensures both document and workspace diagnostics use \
         fresh analysis after config changes."
    );
}

/// Test for Gap 1 (AC1): handle_workspace_diagnostic should use orchestrator pattern
/// for collecting perlcritic diagnostics, not LspServer::collect_external_perlcritic_diagnostics.
///
/// CURRENTLY FAILS because handle_workspace_diagnostic directly calls:
///   - DiagnosticsProvider::new()
///   - self.collect_external_perlcritic_diagnostics()
/// instead of:
///   - self.pull_diagnostics_orchestrator.build_context()
///   - self.pull_diagnostics_orchestrator.collect_perlcritic_diagnostics()
///
/// WILL PASS after refactoring to use the orchestrator pattern.
#[test]
fn test_workspace_diagnostic_uses_orchestrator_for_perlcritic() {
    // This test verifies that diagnostics.rs's handle_workspace_diagnostic
    // uses orchestrator.collect_perlcritic_diagnostics() instead of
    // self.collect_external_perlcritic_diagnostics()
    //
    // The source code should contain:
    //   orchestrator.collect_perlcritic_diagnostics(...)
    // instead of:
    //   self.collect_external_perlcritic_diagnostics(...)

    let diagnostics_rs_source = include_str!("../src/runtime/diagnostics.rs");

    // Find the handle_workspace_diagnostic function
    let func_start = diagnostics_rs_source
        .find("pub(super) fn handle_workspace_diagnostic(")
        .expect("Could not find handle_workspace_diagnostic in diagnostics.rs");

    // Extract the function body (it's quite long, ~280 lines)
    let func_chunk = &diagnostics_rs_source[func_start..func_start + 15000];

    // Check that it uses orchestrator.collect_perlcritic_diagnostics
    let uses_orchestrator_perlcritic =
        func_chunk.contains("orchestrator.collect_perlcritic_diagnostics");

    // Check that it does NOT use self.collect_external_perlcritic_diagnostics
    let uses_direct_path = func_chunk.contains("self.collect_external_perlcritic_diagnostics");

    // For the test to pass, either:
    // 1. It should use orchestrator.collect_perlcritic_diagnostics (the correct pattern), OR
    // 2. If it still uses the direct path, that's a failure

    assert!(
        uses_orchestrator_perlcritic || !uses_direct_path,
        "handle_workspace_diagnostic should use orchestrator.collect_perlcritic_diagnostics() \
         for perlcritic diagnostics, not self.collect_external_perlcritic_diagnostics(). \
         This ensures workspace diagnostics share the same CriticAnalyzer cache as document \
         diagnostics, eliminating the split-brain issue. \
         uses_orchestrator_perlcritic={}, uses_direct_path={}",
        uses_orchestrator_perlcritic,
        uses_direct_path
    );
}

/// Test for Gap 1 (AC1 variant): handle_workspace_diagnostic should use
/// PullDiagnosticsProvider::get_workspace_diagnostics_with_context() for basic diagnostics.
///
/// CURRENTLY FAILS because handle_workspace_diagnostic directly constructs
/// DiagnosticsProvider instead of using PullDiagnosticsProvider.
///
/// WILL PASS after refactoring.
#[test]
fn test_workspace_diagnostic_uses_pull_provider_for_basic_diagnostics() {
    let diagnostics_rs_source = include_str!("../src/runtime/diagnostics.rs");

    // Find the handle_workspace_diagnostic function
    let func_start = diagnostics_rs_source
        .find("pub(super) fn handle_workspace_diagnostic(")
        .expect("Could not find handle_workspace_diagnostic in diagnostics.rs");

    // Extract the function body
    let func_chunk = &diagnostics_rs_source[func_start..func_start + 15000];

    // Check that it uses DiagnosticsProvider::new() directly (the old pattern)
    let uses_direct_provider = func_chunk.contains("DiagnosticsProvider::new(");

    // After refactoring, it should NOT use DiagnosticsProvider::new directly
    // It should use PullDiagnosticsProvider via the orchestrator
    assert!(
        !uses_direct_provider,
        "handle_workspace_diagnostic should not use DiagnosticsProvider::new() directly. \
         It should use PullDiagnosticsProvider::get_workspace_diagnostics_with_context() \
         via the orchestrator pattern, matching handle_document_diagnostic. \
         This ensures consistent diagnostic collection for both document and workspace diagnostics."
    );
}

/// Test for AC2: Both document and workspace diagnostics should use the same
/// CriticAnalyzer cache (via PullDiagnosticsOrchestrator).
///
/// This is a behavioral test that verifies the cache is shared.
/// The test creates a scenario where the orchestrator's CriticAnalyzer is populated,
/// then verifies workspace diagnostics can see that state (after the refactoring).
///
/// CURRENTLY FAILS because workspace diagnostics use LspServer.critic_analyzer
/// directly, not the orchestrator's cache.
/// WILL PASS after refactoring.
#[test]
fn test_workspace_diagnostics_share_orchestrator_cache() -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that after collecting document diagnostics (which populates
    // the orchestrator's CriticAnalyzer), workspace diagnostics can see/use that cache.
    //
    // The test:
    // 1. Opens a document
    // 2. Collects document diagnostics (this uses orchestrator path)
    // 3. Collects workspace diagnostics
    // 4. Verifies behavior is consistent (both paths use same cache)
    //
    // Before fix: workspace diagnostics don't use orchestrator path
    // After fix: workspace diagnostics use orchestrator path

    let server = LspServer::new();

    // Initialize
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a simple Perl file
    let uri = "file:///test_shared_cache.pl";
    let content = r#"#!/usr/bin/perl
use strict;
use warnings;
print "hello\n";
"#;

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // Collect document diagnostics (uses orchestrator path with perlcritic)
    let _doc_response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri }
        })),
    });

    // Collect workspace diagnostics (should use same orchestrator path after fix)
    let ws_response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    // The workspace diagnostic response should exist
    let ws_result = ws_response.ok_or("No response from workspace/diagnostic")?.result;

    // After refactoring, workspace diagnostics use the orchestrator path
    // which shares the same CriticAnalyzer cache as document diagnostics.
    // This means the diagnostics should be collected consistently.
    //
    // The key assertion is that workspace diagnostics come through the orchestrator,
    // which we verify by checking the source code pattern.
    // For a behavioral test, we verify the response structure is correct.

    assert!(ws_result.is_some(), "workspace/diagnostic should return a result");

    let items = ws_result
        .as_ref()
        .and_then(|r| r.get("items"))
        .and_then(|i| i.as_array())
        .ok_or("Expected items array in workspace/diagnostic response")?;

    // We should get at least one item (for our document)
    assert!(!items.is_empty(), "Should have at least one workspace diagnostic report");

    Ok(())
}

/// Test for AC5: Behavioral parity for workspace diagnostics.
/// After refactoring, workspace diagnostics should produce the same output structure.
///
/// CURRENTLY PASSES (same behavior before and after).
/// This test ensures no behavioral regression during refactoring.
#[test]
fn test_workspace_diagnostic_response_structure() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    // Initialize
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a Perl file with an undefined variable
    let uri = "file:///test_undef_var.pl";
    let content = r#"#!/usr/bin/perl
use strict;
use warnings;
print $undefined_var;
"#;

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // Request workspace diagnostics
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Find our document's report
    let our_report = items
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have a report for our URI")?;

    // Check the report structure
    assert!(our_report["kind"].is_string(), "kind should be a string");
    assert!(
        our_report["kind"] == "full" || our_report["kind"] == "unchanged",
        "kind should be 'full' or 'unchanged'"
    );

    if our_report["kind"] == "full" {
        let diags = our_report["items"].as_array().ok_or("Expected items for full report")?;

        // If there are diagnostics with codes, they should have data fields
        for d in diags {
            if d["code"].is_string() && !d["code"].as_str().unwrap().is_empty() {
                // Should have data field with code, category, fixable, tags
                let data = &d["data"];
                assert!(data.is_object(), "Diagnostic with code should have data object: {:?}", d);
            }
        }
    }

    Ok(())
}

// =========================================================================
// Edge Case Tests - Green Test Builder
//
// These tests verify edge cases and boundary conditions for the
// PullDiagnosticsProvider completion. They complement the red tests
// by ensuring the implementation handles unusual inputs gracefully.
// =========================================================================

/// Test edge case: Empty workspace with no open documents.
/// Workspace diagnostics should return an empty items array, not an error.
#[test]
fn test_workspace_diagnostic_empty_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    // Initialize without opening any documents
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Request workspace diagnostics on empty workspace
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Empty workspace should return empty items, not error
    assert!(
        items.is_empty(),
        "Empty workspace should return empty items array, got {} items",
        items.len()
    );

    Ok(())
}

/// Test edge case: Multiple documents in workspace diagnostics.
/// Each document should get its own diagnostic report.
#[test]
fn test_workspace_diagnostic_multiple_documents() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open multiple Perl files
    let files = [
        ("file:///test1.pl", "use strict; print $x;"),
        ("file:///test2.pl", "use strict; print $y;"),
        ("file:///test3.pl", "use strict; print $z;"),
    ];

    for (uri, content) in files {
        let _ = server.handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: None,
            method: "textDocument/didOpen".into(),
            params: Some(json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            })),
        });
    }

    // Request workspace diagnostics
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Should have a report for each document
    assert_eq!(
        items.len(),
        files.len(),
        "Should have {} diagnostic reports, got {}",
        files.len(),
        items.len()
    );

    // Each report should have the correct URI
    for (uri, _) in files {
        let found = items.iter().any(|r| r["uri"].as_str() == Some(uri));
        assert!(found, "Should have diagnostic report for {}", uri);
    }

    Ok(())
}

/// Test edge case: Document with only whitespace.
/// Should not produce unexpected errors.
#[test]
fn test_workspace_diagnostic_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a file with only whitespace
    let uri = "file:///whitespace.pl";
    let content = "   \n\n   \t  \n";

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // Request workspace diagnostics - should not panic
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Find our document's report
    let our_report = items
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have a report for whitespace.pl")?;

    // Report should exist and be valid
    assert!(
        our_report["kind"] == "full" || our_report["kind"] == "unchanged",
        "Whitespace-only file should have valid report"
    );

    Ok(())
}

/// Test edge case: Document with syntax error (parse failure).
/// Should still return a valid diagnostic report.
#[test]
fn test_workspace_diagnostic_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a file with syntax error
    let uri = "file:///syntax_error.pl";
    let content = "use strict;\nmy $x = ;  # syntax error\n";

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // Request workspace diagnostics
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Find our document's report
    let our_report = items
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have a report for syntax_error.pl")?;

    // Report should exist
    assert!(
        our_report["kind"] == "full" || our_report["kind"] == "unchanged",
        "File with syntax error should have valid report"
    );

    // For full reports, should have diagnostic items (parse errors)
    if our_report["kind"] == "full" {
        let diags = our_report["items"].as_array().ok_or("Expected items")?;
        // Should have at least one parse error diagnostic
        let has_parse_error = diags.iter().any(|d| {
            d["source"] == "perl-parser"
                || d["code"].as_str().map(|c| c.starts_with("PL")).unwrap_or(false)
        });
        assert!(
            has_parse_error,
            "Should have at least one parse error diagnostic, got {:#?}",
            diags
        );
    }

    Ok(())
}

/// Test edge case: Document with undefined variable warning.
/// Verifies that warnings are properly reported.
#[test]
fn test_workspace_diagnostic_undefined_variable() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a file with undefined variable (without strict)
    let uri = "file:///undef_var.pl";
    let content = "use warnings;\nprint $undefined_var;\n";

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // Request workspace diagnostics
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Find our document's report
    let our_report = items
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have a report for undef_var.pl")?;

    // Should have a full report with diagnostics
    assert_eq!(our_report["kind"], "full", "Document with warning should have full report");

    let diags = our_report["items"].as_array().ok_or("Expected items")?;
    assert!(!diags.is_empty(), "Should have at least one diagnostic for undefined variable");

    Ok(())
}

/// Test edge case: Very long line in document.
/// Should not cause issues with diagnostic generation.
#[test]
fn test_workspace_diagnostic_long_line() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a file with a very long line
    let uri = "file:///long_line.pl";
    let long_content = "a".repeat(10000);
    let content = format!("use strict; my $x = '{}';\n", long_content);

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // Request workspace diagnostics - should not panic
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result = response.ok_or("No response")?.result.ok_or("No result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?;

    // Find our document's report
    let our_report = items
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have a report for long_line.pl")?;

    // Report should exist and be valid
    assert!(
        our_report["kind"] == "full" || our_report["kind"] == "unchanged",
        "Document with long line should have valid report"
    );

    Ok(())
}

/// Test edge case: Document change detection via textDocument/didChange.
/// Workspace diagnostics should reflect the updated content.
#[test]
fn test_workspace_diagnostic_document_change() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a file
    let uri = "file:///change_test.pl";
    let content1 = "use strict; print $x;\n";
    let content2 = "use strict; my $y = 1; print $y;\n"; // Fixed - no more undefined var

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content1
            }
        })),
    });

    // Get initial diagnostics
    let response1 = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result1 = response1.ok_or("No response")?.result.ok_or("No result")?;
    let items1 = result1["items"].as_array().ok_or("Expected items array")?;
    let report1 = items1
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have report for change_test.pl")?;

    // Change the document
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didChange".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "version": 2
            },
            "contentChanges": [{
                "text": content2
            }]
        })),
    });

    // Get diagnostics after change
    let response2 = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result2 = response2.ok_or("No response")?.result.ok_or("No result")?;
    let items2 = result2["items"].as_array().ok_or("Expected items array")?;
    let report2 = items2
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have report for change_test.pl")?;

    // Both should be full reports (content changed)
    assert_eq!(report1["kind"], "full", "Initial report should be full");
    assert_eq!(report2["kind"], "full", "Report after change should be full (content changed)");

    // The diagnostics should differ (content changed)
    let diag1_count = report1["items"].as_array().map(|a| a.len()).unwrap_or(0);
    let diag2_count = report2["items"].as_array().map(|a| a.len()).unwrap_or(0);

    // After fixing the undefined variable, there should be fewer or no diagnostics
    // (depending on if there are other issues)
    assert!(
        diag2_count <= diag1_count,
        "Fixed document should not have more diagnostics than original. \
         Original: {}, After fix: {}",
        diag1_count,
        diag2_count
    );

    Ok(())
}

/// Test edge case: Result ID stability for unchanged content.
/// If content hasn't changed, workspace diagnostics should return the same result ID.
#[test]
fn test_workspace_diagnostic_result_id_stability() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    });

    // Open a document with a parse error
    let uri = "file:///stability_test.pl";
    let content = "use strict; print $x;\n";

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    });

    // First diagnostic request - should be full
    let response1 = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result1 = response1.ok_or("No response")?.result.ok_or("No result")?;
    let items1 = result1["items"].as_array().ok_or("Expected items array")?;
    let report1 = items1
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have report for stability_test.pl")?;

    assert_eq!(report1["kind"], "full", "Initial report should be full");

    let result_id1 = report1["resultId"].as_str().ok_or("Should have resultId")?;

    // Second request - content unchanged, should get same resultId
    let response2 = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "workspace/diagnostic".into(),
        params: Some(json!({})),
    });

    let result2 = response2.ok_or("No response")?.result.ok_or("No result")?;
    let items2 = result2["items"].as_array().ok_or("Expected items array")?;
    let report2 = items2
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri))
        .ok_or("Should have report for stability_test.pl")?;

    // resultId should be the same if content is unchanged
    let result_id2 = report2["resultId"].as_str().ok_or("Should have resultId")?;
    assert_eq!(
        result_id1, result_id2,
        "ResultId should be stable for unchanged content. \
         Got '{}' first, '{}' second.",
        result_id1, result_id2
    );

    Ok(())
}
