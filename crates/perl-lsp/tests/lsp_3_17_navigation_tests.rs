//! LSP 3.17 Navigation Contract Tests
//!
//! Tests for hover, signatureHelp, declaration, definition, typeDefinition,
//! implementation, references, documentHighlight, and documentSymbol.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== LANGUAGE FEATURES: NAVIGATION ====================

#[test]
fn test_hover_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "print 'hello'")?;

    let response = harness.request(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 0 },
            "workDoneToken": "hover-1"  // optional progress token
        }),
    )?;

    if !response.is_null() {
        assert!(
            response["contents"].is_string()
                || response["contents"].is_object()
                || response["contents"].is_array()
        );
    }
    Ok(())
}

#[test]
fn test_signature_help_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "substr(")?;

    let response = harness.request(
        "textDocument/signatureHelp",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 7 },
            "context": {
                "triggerKind": 1,  // Invoked
                "triggerCharacter": "(",
                "isRetrigger": false,
                "activeSignatureHelp": null
            }
        }),
    )?;

    if !response.is_null() {
        assert!(response["signatures"].is_array());
    }
    Ok(())
}

#[test]
fn test_declaration_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $x = 1;\n$x")?;

    let response = harness.request(
        "textDocument/declaration",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 0 }
        }),
    )?;

    // Can be Location, Location[], LocationLink[], or null
    assert!(response.is_null() || response.is_object() || response.is_array());
    Ok(())
}

#[test]
fn test_definition_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "sub test {}\ntest()")?;

    let response = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 0 }
        }),
    )?;

    // Can be Location, Location[], LocationLink[], or null
    assert!(response.is_null() || response.is_object() || response.is_array());
    Ok(())
}

#[test]
fn test_type_definition_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;
    let caps = &init_response["capabilities"];

    // Check if server advertises typeDefinition support
    let supported =
        caps.get("typeDefinitionProvider").is_some() && !caps["typeDefinitionProvider"].is_null();

    harness.open("file:///test.pl", "my $obj = bless {}, 'MyClass'")?;

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/typeDefinition",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 }
        }
    }));

    if supported {
        // If supported, should return a result (which could be an array or locations)
        // The test harness returns the result directly, not wrapped in a response
        assert!(
            response.is_array() || response.is_object() || response.is_null(),
            "Expected array, object, or null result for typeDefinition"
        );
    } else {
        // If not supported, should return an error (MethodNotFound or InternalError)
        assert!(
            response.get("error").is_some(),
            "Expected error when not advertised"
        );
        let error_code = response["error"]["code"]
            .as_i64()
            .ok_or("error code not i64")?;
        assert!(
            error_code == -32601 || error_code == -32603,
            "Expected MethodNotFound (-32601) or InternalError (-32603), got {}",
            error_code
        );
    }
    Ok(())
}

#[test]
fn test_implementation_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;
    let caps = &init_response["capabilities"];

    // Check if server advertises implementation support
    let supported =
        caps.get("implementationProvider").is_some() && !caps["implementationProvider"].is_null();

    harness.open(
        "file:///test.pl",
        "package Base;\nsub method {}\npackage Derived;\nuse base 'Base';",
    )?;

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/implementation",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 1, "character": 4 }
        }
    }));

    if supported {
        // If supported, should return a result (which could be an array or locations)
        // The test harness returns the result directly, not wrapped in a response
        assert!(
            response.is_array() || response.is_object(),
            "Expected array or object result for implementation"
        );
    } else {
        // If not supported, should return an error (MethodNotFound or InternalError)
        assert!(
            response.get("error").is_some(),
            "Expected error when not advertised"
        );
        let error_code = response["error"]["code"]
            .as_i64()
            .ok_or("error code not i64")?;
        assert!(
            error_code == -32601 || error_code == -32603,
            "Expected MethodNotFound (-32601) or InternalError (-32603), got {}",
            error_code
        );
    }
    Ok(())
}

#[test]
fn test_references_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $x = 1;\n$x++;\nprint $x;")?;

    let response = harness.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 },
            "context": {
                "includeDeclaration": true
            }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

#[test]
fn test_document_highlight_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "my $x = 1;\n$x = 2;\nprint $x;")?;

    let response = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 0, "character": 4 }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

#[test]
fn test_document_symbol_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open("file:///test.pl", "package Foo;\nsub bar {}\nmy $var = 1;")?;

    let response = harness.request(
        "textDocument/documentSymbol",
        json!({
            "textDocument": { "uri": "file:///test.pl" }
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}
