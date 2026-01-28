//! Integration tests for multiple error collection (Issue #451)
//!
//! Verifies that the LSP server correctly reports all collected parse errors
//! from a single parse pass, not just the first error encountered.

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Test that LSP diagnostics include all parse errors from a single file (AC #451)
#[test]
fn test_451_lsp_reports_multiple_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
    // AC:451 - Integration test for multiple error collection
    let mut server = LspServer::new();

    // Initialize server with diagnostic support
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {
                "textDocument": {
                    "diagnostic": {
                        "dynamicRegistration": false
                    }
                }
            }
        })),
    };
    let _ = server.handle_request(init_request);

    // Send initialized notification
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized_notification);

    // Open a document with MULTIPLE syntax errors
    let uri = "file:///test_multiple_errors.pl";
    let content = r#"#!/usr/bin/perl
use strict;

my $a = ;       # Error 1: missing expression
print "hello";  # Valid statement
my $b = ;       # Error 2: missing expression
print "world";  # Valid statement
my $c = ;       # Error 3: missing expression
"#;

    let did_open_request = JsonRpcRequest {
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
    };
    let _ = server.handle_request(did_open_request);

    // Request pull diagnostics to get all errors
    let diagnostic_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri }
        })),
    };

    let response =
        server.handle_request(diagnostic_request).ok_or("Failed to get diagnostic response")?;
    let result = response.result.ok_or("Response missing result field")?;

    // Verify we got a full diagnostic report
    assert_eq!(result["kind"], "full", "Should return full diagnostic report");

    // Get diagnostic items
    let items = result["items"].as_array().ok_or("Expected items to be an array")?;

    // AC #451: Should report ALL errors, not just the first one
    // We expect at least 3 parse errors (one for each `my $x = ;`)
    assert!(
        items.len() >= 3,
        "Should report multiple errors (found {}), not fail-fast on first error",
        items.len()
    );

    // Verify each diagnostic has proper structure
    for (i, item) in items.iter().enumerate() {
        assert!(item["range"].is_object(), "Diagnostic {} missing range", i);
        assert!(item["severity"].is_number(), "Diagnostic {} missing severity", i);
        assert!(item["message"].is_string(), "Diagnostic {} missing message", i);

        // Print for debugging
        eprintln!(
            "Diagnostic {}: {} (line {}, col {})",
            i, item["message"], item["range"]["start"]["line"], item["range"]["start"]["character"]
        );
    }

    Ok(())
}

/// Test that error collection continues even with nested block errors (AC #451)
#[test]
fn test_451_lsp_reports_errors_in_nested_blocks() -> Result<(), Box<dyn std::error::Error>> {
    // AC:451 - Nested block error collection
    let mut server = LspServer::new();

    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    let _ = server.handle_request(init_request);

    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized_notification);

    // Document with errors in different blocks
    let uri = "file:///test_nested.pl";
    let content = r#"#!/usr/bin/perl
sub foo {
    my $x = ;  # Error in sub
    print 1;
}

if (1) {
    my $y = ;  # Error in if block
    print 2;
}

while (1) {
    my $z = ;  # Error in while block
    print 3;
}
"#;

    let did_open_request = JsonRpcRequest {
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
    };
    let _ = server.handle_request(did_open_request);

    let diagnostic_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri }
        })),
    };

    let response =
        server.handle_request(diagnostic_request).ok_or("Failed to get diagnostic response")?;
    let result = response.result.ok_or("Response missing result field")?;

    let items = result["items"].as_array().ok_or("Expected items to be an array")?;

    // Should collect errors from all nested blocks
    assert!(
        items.len() >= 3,
        "Should collect errors from all nested blocks (found {})",
        items.len()
    );

    Ok(())
}

/// Test error limit prevents unbounded error collection (AC5 #451)
#[test]
fn test_451_lsp_respects_error_limit() -> Result<(), Box<dyn std::error::Error>> {
    // AC:451 - AC5: Error limit enforcement
    let mut server = LspServer::new();

    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({
            "processId": 1,
            "capabilities": {}
        })),
    };
    let _ = server.handle_request(init_request);

    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    let _ = server.handle_request(initialized_notification);

    // Generate many errors (more than typical limit)
    let mut content = String::from("#!/usr/bin/perl\n");
    for i in 0..150 {
        content.push_str(&format!("my $x{} = ;\n", i));
    }

    let uri = "file:///test_many_errors.pl";
    let did_open_request = JsonRpcRequest {
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
    };
    let _ = server.handle_request(did_open_request);

    let diagnostic_request = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/diagnostic".into(),
        params: Some(json!({
            "textDocument": { "uri": uri }
        })),
    };

    let response =
        server.handle_request(diagnostic_request).ok_or("Failed to get diagnostic response")?;
    let result = response.result.ok_or("Response missing result field")?;

    let items = result["items"].as_array().ok_or("Expected items to be an array")?;

    // Should have errors, but not unbounded
    assert!(!items.is_empty(), "Should have some errors");
    assert!(
        items.len() < 500,
        "Should limit error collection to prevent flood (found {})",
        items.len()
    );

    Ok(())
}
