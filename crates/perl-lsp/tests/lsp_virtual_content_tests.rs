#![allow(clippy::unwrap_used, clippy::expect_used)]
//! LSP Virtual Content Tests
//!
//! Tests for workspace/textDocumentContent LSP 3.18 feature
//! for serving virtual documents like perldoc:// URIs.

use parking_lot::Mutex;
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;
use std::sync::Arc;

/// Helper to send a request and get result
fn send_request(
    server: &mut LspServer,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: method.to_string(),
        params: Some(params),
    };

    let response = server.handle_request(request).expect("Should get response");
    if let Some(error) = response.error {
        return Err(error.message);
    }
    response.result.ok_or_else(|| "No result".to_string())
}

/// Create a test server
fn setup_server() -> LspServer {
    let output = Arc::new(Mutex::new(Box::new(Vec::new()) as Box<dyn std::io::Write + Send>));
    let mut server = LspServer::with_output(output);

    // Initialize the server
    send_request(
        &mut server,
        "initialize",
        json!({
            "capabilities": {},
            "processId": 12345,
            "rootUri": "file:///test"
        }),
    )
    .ok();

    // Send initialized notification
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_request);

    server
}

#[test]
fn lsp_virtual_perldoc_strict() {
    let mut server = setup_server();

    let result = send_request(
        &mut server,
        "workspace/textDocumentContent",
        json!({
            "uri": "perldoc://strict"
        }),
    );

    // This test may fail if perldoc is not available or strict module is not installed
    match result {
        Ok(content) => {
            let text = content["text"].as_str().expect("Should have text field");
            assert!(!text.is_empty(), "Content should not be empty");
            // Verify it contains documentation-like content
            assert!(
                text.to_lowercase().contains("strict") || text.to_lowercase().contains("pragma"),
                "Content should mention 'strict' or 'pragma'"
            );
        }
        Err(e) => {
            // If perldoc is not available, this is expected
            eprintln!("Note: perldoc may not be available: {}", e);
            assert!(
                e.contains("not found") || e.contains("Unsupported"),
                "Error should indicate missing perldoc or unsupported URI"
            );
        }
    }
}

#[test]
fn lsp_virtual_perldoc_invalid_module() {
    let mut server = setup_server();

    let result = send_request(
        &mut server,
        "workspace/textDocumentContent",
        json!({
            "uri": "perldoc://ThisModuleDefinitelyDoesNotExist12345"
        }),
    );

    // Should return error for non-existent module
    assert!(result.is_err(), "Should return error for non-existent module");
}

#[test]
fn lsp_virtual_unsupported_scheme() {
    let mut server = setup_server();

    let result = send_request(
        &mut server,
        "workspace/textDocumentContent",
        json!({
            "uri": "unsupported://some/path"
        }),
    );

    // Should return error for unsupported scheme
    assert!(result.is_err(), "Should return error for unsupported scheme");
    let error = result.unwrap_err();
    assert!(
        error.contains("Unsupported") || error.contains("not found"),
        "Error message should indicate unsupported URI"
    );
}

#[test]
fn lsp_virtual_missing_params() {
    let mut server = setup_server();

    // Empty params
    let result = send_request(&mut server, "workspace/textDocumentContent", json!({}));
    assert!(result.is_err(), "Should return error for missing URI");
}

#[test]
fn lsp_virtual_perldoc_common_modules() {
    let mut server = setup_server();

    // Test common modules that should be available on most systems
    let modules = vec!["warnings", "vars"];

    for module in modules {
        let result = send_request(
            &mut server,
            "workspace/textDocumentContent",
            json!({
                "uri": format!("perldoc://{}", module)
            }),
        );

        match result {
            Ok(content) => {
                let text = content["text"].as_str().expect("Should have text field");
                assert!(text.len() > 100, "Module {} should have substantial content", module);
            }
            Err(_) => {
                // If perldoc is not available, skip this test
                eprintln!(
                    "Note: Skipping test for module '{}' - perldoc may not be available",
                    module
                );
                return;
            }
        }
    }
}

#[test]
fn lsp_virtual_empty_module_name() {
    let mut server = setup_server();

    let result = send_request(
        &mut server,
        "workspace/textDocumentContent",
        json!({
            "uri": "perldoc://"
        }),
    );

    // Should return error for empty module name
    assert!(result.is_err(), "Should return error for empty module name");
}
