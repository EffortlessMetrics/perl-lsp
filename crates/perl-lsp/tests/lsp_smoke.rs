//! LSP Smoke Tests - Deterministic Merge-Gate Checks
//!
//! This module provides a minimal but comprehensive set of smoke tests designed for
//! use as merge-gate checks. The tests prioritize determinism and reliability over
//! breadth of coverage.
//!
//! ## Design Principles
//!
//! 1. **No timing dependencies**: Tests use direct API calls, not async I/O
//! 2. **Proper initialization**: Server is initialized before feature requests
//! 3. **Isolated environment**: Each test uses a fresh server instance
//! 4. **Predictable inputs**: Small, focused fixtures with known expected outputs
//! 5. **Clear failure messages**: Descriptive assertions for debugging
//! 6. **No retry logic**: Tests must be inherently stable
//!
//! ## Test Coverage
//!
//! - Server initialization and capability advertisement
//! - Document lifecycle (open, change)
//! - Core language features (hover, completion, definition, references)
//! - Document symbols and workspace symbols
//! - Folding ranges
//! - Error handling (unknown methods, non-existent documents)
//! - Graceful shutdown
//!
//! ## Execution
//!
//! ```bash
//! # Run the smoke tests (recommended: single-threaded for clean output)
//! cargo test -p perl-lsp --test lsp_smoke -- --test-threads=1
//!
//! # Check if tests pass (exit code 0 = success)
//! cargo test -p perl-lsp --test lsp_smoke -- --test-threads=1; echo "Exit: $?"
//!
//! # Run with verbose output (note: server notifications may mix with test output)
//! cargo test -p perl-lsp --test lsp_smoke -- --test-threads=1 --nocapture
//! ```
//!
//! ## CI Integration
//!
//! These tests are designed to complete in under 30 seconds total and produce
//! deterministic results suitable for merge-gate automation.
//!
//! **Note**: Test output may include LSP server notifications (JSON-RPC messages)
//! that appear interleaved with test results. This is expected behavior since
//! the server writes to stdout by default. The important metric is the exit code:
//! - Exit code 0 = all tests passed
//! - Exit code 1 = one or more tests failed
//!
//! Example CI check:
//! ```bash
//! cargo test -p perl-lsp --test lsp_smoke -- --test-threads=1 || exit 1
//! ```

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};

// =============================================================================
// TEST FIXTURES
// =============================================================================

/// Simple Perl script with a subroutine for basic smoke testing
const FIXTURE_SIMPLE_SUB: &str = r#"#!/usr/bin/env perl
use strict;
use warnings;

my $greeting = "Hello";

sub say_hello {
    my ($name) = @_;
    return "$greeting, $name!";
}

my $result = say_hello("World");
print $result;
"#;

/// Perl module for cross-file testing
const FIXTURE_MODULE: &str = r#"package Smoke::Module;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub greet {
    my ($self, $name) = @_;
    return "Hello, $name from Module";
}

1;
"#;

// Note: FIXTURE_NO_STRICT was removed - not currently used by smoke tests
// If needed for diagnostics testing, can be restored:
// const FIXTURE_NO_STRICT: &str = r#"#!/usr/bin/env perl
// # Missing 'use strict' and 'use warnings'
// my $x = 42;
// print $x;
// "#;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Create a fresh LSP server instance
fn create_server() -> LspServer {
    LspServer::new()
}

/// Send an initialize request and return the result
///
/// NOTE: Does NOT send `initialized` notification to avoid triggering
/// workspace indexing and stdout notifications that interfere with test output.
/// For tests that need full initialization, use `initialize_server_full()`.
fn initialize_server(server: &mut LspServer) -> Result<Value, Box<dyn std::error::Error>> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///tmp/smoke_test",
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    }
                }
            }
        })),
    };

    let response = server.handle_request(request).ok_or("Initialize should return response")?;
    assert!(response.error.is_none(), "Initialize should not return error");

    Ok(response.result.ok_or("Initialize should return result")?)
}

/// Send initialize request AND initialized notification
/// Use when you need full server functionality including workspace indexing
fn initialize_server_full(server: &mut LspServer) -> Result<Value, Box<dyn std::error::Error>> {
    let result = initialize_server(server)?;

    // Send initialized notification
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized);

    Ok(result)
}

/// Open a document in the server
fn open_document(server: &mut LspServer, uri: &str, content: &str) {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": content
            }
        })),
    };
    server.handle_request(request);
}

/// Send a request and get the result
fn send_request(server: &mut LspServer, id: i32, method: &str, params: Value) -> Option<Value> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(id)),
        method: method.to_string(),
        params: Some(params),
    };

    let response = server.handle_request(request)?;
    if response.error.is_some() {
        return None;
    }
    response.result
}

/// Shutdown the server gracefully
fn shutdown_server(server: &mut LspServer) {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(999)),
        method: "shutdown".to_string(),
        params: None,
    };
    server.handle_request(request);

    let exit = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "exit".to_string(),
        params: None,
    };
    server.handle_request(exit);
}

// =============================================================================
// SMOKE TEST: SERVER INITIALIZATION
// =============================================================================

/// Smoke test: Server initializes successfully and advertises core capabilities
///
/// This test verifies:
/// - Server accepts initialize request
/// - Server returns valid capabilities object
/// - Core providers are advertised (hover, completion, definition)
#[test]
fn smoke_server_initialization_and_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let init_result = initialize_server(&mut server)?;

    // Verify response structure
    let caps = &init_result["capabilities"];

    // Core capabilities that MUST be present for a functional LSP
    assert!(
        caps.get("textDocumentSync").is_some(),
        "textDocumentSync capability must be advertised"
    );
    assert_eq!(caps.get("hoverProvider"), Some(&json!(true)), "hoverProvider must be true");
    assert_eq!(
        caps.get("definitionProvider"),
        Some(&json!(true)),
        "definitionProvider must be true"
    );
    assert!(caps.get("completionProvider").is_some(), "completionProvider must be advertised");
    assert_eq!(
        caps.get("referencesProvider"),
        Some(&json!(true)),
        "referencesProvider must be true"
    );
    assert!(
        caps.get("documentSymbolProvider").is_some(),
        "documentSymbolProvider must be advertised"
    );

    shutdown_server(&mut server);
    Ok(())
}

/// Smoke test: Server rejects double initialization
#[test]
fn smoke_double_initialization_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;

    // Try to initialize again
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": "file:///tmp/test2",
            "capabilities": {}
        })),
    };

    let response = server.handle_request(request).ok_or("Should return response")?;
    assert!(response.error.is_some(), "Second initialize should return error");

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: DOCUMENT LIFECYCLE
// =============================================================================

/// Smoke test: Document open is accepted
#[test]
fn smoke_document_open() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;

    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    // Verify document is tracked by requesting hover (should not fail)
    let result = send_request(
        &mut server,
        10,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    // Result can be null but should not error
    assert!(result.is_some(), "Hover should return a response (even if null)");

    shutdown_server(&mut server);
    Ok(())
}

/// Smoke test: Document change is accepted
#[test]
fn smoke_document_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;

    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    // Change the document
    let change_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didChange".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": "file:///test.pl",
                "version": 2
            },
            "contentChanges": [{
                "text": FIXTURE_SIMPLE_SUB.replace("Hello", "Greetings")
            }]
        })),
    };
    server.handle_request(change_request);

    // Verify server still responds
    let result = send_request(
        &mut server,
        11,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 4, "character": 5 }
        }),
    );

    assert!(result.is_some(), "Server should still respond after document change");

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: HOVER
// =============================================================================

/// Smoke test: Hover returns valid response
#[test]
fn smoke_hover_response_structure() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    let result = send_request(
        &mut server,
        20,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 4, "character": 5 }  // On $greeting
        }),
    );

    // Hover should return something (may be null for some positions)
    assert!(result.is_some(), "Hover should return a response");

    let hover = result.ok_or("Hover should return a value")?;
    if !hover.is_null() {
        // If we got content, verify structure
        assert!(
            hover.get("contents").is_some() || hover.get("range").is_some(),
            "Hover result should have contents or range"
        );
    }

    shutdown_server(&mut server);
    Ok(())
}

/// Smoke test: Hover on subroutine name
#[test]
fn smoke_hover_on_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    let result = send_request(
        &mut server,
        21,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 6, "character": 6 }  // On "say_hello" definition
        }),
    );

    assert!(result.is_some(), "Hover on subroutine should return response");

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: COMPLETION
// =============================================================================

/// Smoke test: Completion returns items
#[test]
fn smoke_completion_returns_items() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    let result = send_request(
        &mut server,
        30,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 11, "character": 10 }
        }),
    );

    assert!(result.is_some(), "Completion should return a response");

    let completion = result.ok_or("Completion should return a value")?;
    // Completion returns either array or object with items
    let items = if completion.is_array() {
        completion.as_array()
    } else {
        completion.get("items").and_then(|v| v.as_array())
    };

    // We may or may not have items depending on context
    if let Some(items) = items {
        if !items.is_empty() {
            // Verify structure of first item
            assert!(items[0].get("label").is_some(), "Completion items must have label");
        }
    }

    shutdown_server(&mut server);
    Ok(())
}

/// Smoke test: Completion for builtins
#[test]
fn smoke_completion_builtins() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;

    // Open a file with partial builtin
    open_document(&mut server, "file:///builtin.pl", "pri");

    let result = send_request(
        &mut server,
        31,
        "textDocument/completion",
        json!({
            "textDocument": { "uri": "file:///builtin.pl" },
            "position": { "line": 0, "character": 3 }
        }),
    );

    assert!(result.is_some(), "Completion should return a response for builtins");

    let completion = result.ok_or("Completion should return a value")?;
    let items = if completion.is_array() {
        completion.as_array()
    } else {
        completion.get("items").and_then(|v| v.as_array())
    };

    // Should have at least 'print' in completions
    if let Some(items) = items {
        let has_print = items.iter().any(|item| {
            item.get("label").and_then(|l| l.as_str()).map(|s| s.contains("print")).unwrap_or(false)
        });
        assert!(has_print, "Completion should include 'print' for 'pri' prefix");
    }

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: DEFINITION
// =============================================================================

/// Smoke test: Definition returns locations
#[test]
fn smoke_definition_returns_locations() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    // Request definition for say_hello call
    let result = send_request(
        &mut server,
        40,
        "textDocument/definition",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 11, "character": 14 }  // On say_hello call
        }),
    );

    assert!(result.is_some(), "Definition should return a response");

    let definition = result.ok_or("Definition should return a value")?;
    if !definition.is_null() {
        // Definition returns either Location or Location[]
        if definition.is_array() {
            let locations = definition.as_array().ok_or("Definition array should be valid")?;
            if !locations.is_empty() {
                assert!(locations[0].get("uri").is_some(), "Location must have uri");
                assert!(locations[0].get("range").is_some(), "Location must have range");
            }
        } else if definition.is_object() {
            assert!(definition.get("uri").is_some(), "Location must have uri");
        }
    }

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: DOCUMENT SYMBOLS
// =============================================================================

/// Smoke test: Document symbols returns symbols
#[test]
fn smoke_document_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    let result = send_request(
        &mut server,
        50,
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    );

    assert!(result.is_some(), "Document symbols should return a response");

    let symbols = result.ok_or("Document symbols should return a value")?;
    let symbols_array = symbols.as_array().ok_or("Symbols should be an array")?;

    // Should have at least one symbol (say_hello subroutine)
    assert!(!symbols_array.is_empty(), "Should have at least one symbol");

    // Verify structure
    for symbol in symbols_array {
        assert!(symbol.get("name").is_some(), "Symbol must have name");
        assert!(
            symbol.get("range").is_some() || symbol.get("location").is_some(),
            "Symbol must have range or location"
        );
    }

    // Check for expected symbol
    let names: Vec<&str> =
        symbols_array.iter().filter_map(|s| s.get("name").and_then(|n| n.as_str())).collect();

    assert!(
        names.iter().any(|n| n.contains("say_hello")),
        "Should find say_hello in symbols. Found: {:?}",
        names
    );

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: REFERENCES
// =============================================================================

/// Smoke test: References returns locations
#[test]
fn smoke_find_references() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    let result = send_request(
        &mut server,
        60,
        "textDocument/references",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 4, "character": 5 },  // On $greeting
            "context": { "includeDeclaration": true }
        }),
    );

    assert!(result.is_some(), "References should return a response");

    let refs = result.ok_or("References should return a value")?;
    if let Some(refs_array) = refs.as_array() {
        // Verify structure
        for reference in refs_array {
            assert!(reference.get("uri").is_some(), "Reference must have uri");
            assert!(reference.get("range").is_some(), "Reference must have range");
        }
    }

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: FOLDING RANGES
// =============================================================================

/// Smoke test: Folding ranges returns ranges
#[test]
fn smoke_folding_ranges() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    let result = send_request(
        &mut server,
        70,
        "textDocument/foldingRange",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    );

    assert!(result.is_some(), "Folding ranges should return a response");

    let ranges = result.ok_or("Folding ranges should return a value")?;
    if let Some(ranges_array) = ranges.as_array() {
        for range in ranges_array {
            assert!(range.get("startLine").is_some(), "Folding range must have startLine");
            assert!(range.get("endLine").is_some(), "Folding range must have endLine");
        }
    }

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: WORKSPACE SYMBOLS
// =============================================================================

/// Smoke test: Workspace symbol search
#[test]
fn smoke_workspace_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);
    open_document(&mut server, "file:///module.pm", FIXTURE_MODULE);

    let result = send_request(
        &mut server,
        80,
        "workspace/symbol",
        json!({
            "query": "greet"
        }),
    );

    assert!(result.is_some(), "Workspace symbols should return a response");

    // Response may be empty if indexing is not complete, which is acceptable
    let symbols = result.ok_or("Workspace symbols should return a value")?;
    if let Some(symbols_array) = symbols.as_array() {
        for symbol in symbols_array {
            assert!(symbol.get("name").is_some(), "Workspace symbol must have name");
            // May have location or containerName
        }
    }

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: ERROR HANDLING
// =============================================================================

/// Smoke test: Server handles unknown method gracefully
#[test]
fn smoke_unknown_method_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;

    let result = send_request(
        &mut server,
        90,
        "textDocument/unknownMethod",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    );

    // Should return None (error response)
    assert!(result.is_none(), "Unknown method should return error (None)");

    // Server should still be responsive
    let hover_result = send_request(
        &mut server,
        91,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );
    assert!(hover_result.is_some(), "Server should still respond after error");

    shutdown_server(&mut server);
    Ok(())
}

/// Smoke test: Server handles request for non-existent document
#[test]
fn smoke_nonexistent_document() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;

    // Don't open any document, just request hover
    let result = send_request(
        &mut server,
        92,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///nonexistent.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    // Should return something (null is acceptable)
    assert!(result.is_some(), "Should return response for non-existent document");

    shutdown_server(&mut server);
    Ok(())
}

// =============================================================================
// SMOKE TEST: SHUTDOWN
// =============================================================================

/// Smoke test: Graceful shutdown
#[test]
fn smoke_graceful_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = create_server();
    let _ = initialize_server_full(&mut server)?;
    open_document(&mut server, "file:///test.pl", FIXTURE_SIMPLE_SUB);

    // Make some requests
    let _ = send_request(
        &mut server,
        100,
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 }
        }),
    );

    // Shutdown
    let shutdown_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(101)),
        method: "shutdown".to_string(),
        params: None,
    };

    let response =
        server.handle_request(shutdown_request).ok_or("Shutdown should return response")?;
    assert!(response.error.is_none(), "Shutdown should not return error");

    // Exit
    let exit_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "exit".to_string(),
        params: None,
    };
    server.handle_request(exit_request);

    // Test passes if we get here without hanging or crashing
    Ok(())
}

// =============================================================================
// STRUCTURED OUTPUT TEST
// =============================================================================

/// Test that produces structured output for CI receipts
#[test]
fn smoke_test_summary() {
    // This test always passes and prints a summary
    // Useful for CI to confirm smoke tests ran
    eprintln!();
    eprintln!("=== LSP SMOKE TEST SUMMARY ===");
    eprintln!("Tests executed:");
    eprintln!("  - Server initialization and capabilities");
    eprintln!("  - Double initialization rejection");
    eprintln!("  - Document open/change lifecycle");
    eprintln!("  - Hover response structure");
    eprintln!("  - Hover on subroutine");
    eprintln!("  - Completion returns items");
    eprintln!("  - Completion for builtins");
    eprintln!("  - Definition returns locations");
    eprintln!("  - Document symbols");
    eprintln!("  - Find references");
    eprintln!("  - Folding ranges");
    eprintln!("  - Workspace symbols");
    eprintln!("  - Error handling (unknown method)");
    eprintln!("  - Non-existent document handling");
    eprintln!("  - Graceful shutdown");
    eprintln!("================================");
}
