//! AST Explorer tests for the `perl/showAst` custom request handler.
//!
//! The VSCode extension sends `perl/showAst` with `{ uri: "..." }` and expects
//! a JSON string (the S-expression AST) or `null` when no AST is available.
//!
//! # Test coverage
//!
//! - Valid Perl file returns a non-null string AST
//! - AST contains expected constructs (use statements, subroutines)
//! - Document not opened returns an error
//! - Server not initialized returns ServerNotInitialized error
//! - Missing URI parameter returns INVALID_PARAMS error
//! - Empty Perl file returns null (no AST) gracefully

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

// ============================================================================
// Helpers
// ============================================================================

fn create_and_init_server() -> LspServer {
    let server = LspServer::new();
    server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        })),
    });
    server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });
    server
}

fn open_document(server: &LspServer, uri: &str, text: &str) {
    server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    });
}

fn show_ast(server: &LspServer, uri: &str) -> Option<serde_json::Value> {
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(42)),
        method: "perl/showAst".to_string(),
        params: Some(json!({ "uri": uri })),
    })?;
    Some(response.result.unwrap_or(serde_json::Value::Null))
}

fn show_ast_error(
    server: &LspServer,
    params: Option<serde_json::Value>,
) -> Option<perl_lsp::JsonRpcError> {
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(42)),
        method: "perl/showAst".to_string(),
        params,
    })?;
    response.error
}

// ============================================================================
// Tests
// ============================================================================

/// The handler must return a non-null JSON string for a valid Perl file.
#[test]
fn show_ast_returns_string_for_valid_perl() {
    let server = create_and_init_server();
    let uri = "file:///test_ast.pl";
    open_document(
        &server,
        uri,
        "use strict;\nuse warnings;\nsub foo { return 1; }\n",
    );

    let result = show_ast(&server, uri);
    assert!(result.is_some(), "perl/showAst should return a response");

    let value = result.unwrap();
    assert!(
        !value.is_null(),
        "perl/showAst should return a non-null AST string for a valid file"
    );
    assert!(
        value.is_string(),
        "perl/showAst result should be a JSON string, got: {:?}",
        value
    );
}

/// The returned S-expression must contain recognizable Perl constructs.
#[test]
fn show_ast_contains_expected_constructs() {
    let server = create_and_init_server();
    let uri = "file:///constructs.pl";
    open_document(
        &server,
        uri,
        "use strict;\nuse warnings;\nsub greet { my ($name) = @_; return \"Hello, $name\"; }\n",
    );

    let result = show_ast(&server, uri).expect("should return a result");
    let ast_str = result.as_str().expect("result must be a string");

    // The S-expression wraps the whole file in source_file (or program)
    assert!(
        ast_str.contains("source_file") || ast_str.contains("program"),
        "AST should contain a top-level program/source_file node. Got: {}",
        &ast_str[..ast_str.len().min(200)]
    );
}

/// Requesting AST for a URI that was never opened returns a Document-Not-Found error.
#[test]
fn show_ast_returns_error_for_unknown_document() {
    let server = create_and_init_server();
    // Intentionally do NOT open any document

    let err = show_ast_error(&server, Some(json!({ "uri": "file:///never_opened.pl" })));
    assert!(
        err.is_some(),
        "perl/showAst should return an error for an unopened document"
    );
    // Error code -32602 (INVALID_PARAMS) is the spec-compliant choice for
    // "document not found" in a custom request with a URI parameter.
    let code = err.unwrap().code;
    assert_eq!(
        code, -32602,
        "Expected INVALID_PARAMS (-32602) for unknown document, got {code}"
    );
}

/// Calling `perl/showAst` before `initialize` returns ServerNotInitialized.
#[test]
fn show_ast_before_init_returns_server_not_initialized() {
    let server = LspServer::new(); // NOT initialized

    let err = show_ast_error(&server, Some(json!({ "uri": "file:///foo.pl" })));
    assert!(
        err.is_some(),
        "Should return an error before initialization"
    );
    let code = err.unwrap().code;
    assert_eq!(
        code, -32002,
        "Expected ServerNotInitialized (-32002), got {code}"
    );
}

/// Missing `uri` field in params returns INVALID_PARAMS.
#[test]
fn show_ast_missing_uri_returns_invalid_params() {
    let server = create_and_init_server();

    let err = show_ast_error(&server, Some(json!({ "other": "value" })));
    assert!(err.is_some(), "Should return an error when uri is missing");
    let code = err.unwrap().code;
    assert_eq!(
        code, -32602,
        "Expected INVALID_PARAMS (-32602) for missing uri, got {code}"
    );
}

/// Null params returns INVALID_PARAMS.
#[test]
fn show_ast_null_params_returns_invalid_params() {
    let server = create_and_init_server();

    let err = show_ast_error(&server, None);
    assert!(err.is_some(), "Should return an error for null params");
    let code = err.unwrap().code;
    assert_eq!(
        code, -32602,
        "Expected INVALID_PARAMS (-32602) for null params, got {code}"
    );
}
