//! Tests for documentLink/resolve LSP feature
//!
//! Tests the deferred resolution pattern where initial documentLink returns
//! links with data fields, and documentLink/resolve fills in the target.

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn data_str<'a>(link: &'a Value, pointer: &str) -> Option<&'a str> {
    link.pointer(pointer).and_then(Value::as_str)
}

fn initialized_server() -> LspServer {
    let server = LspServer::new();
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    server
}

/// Test that documentLink/resolve returns target for deferred module links
#[test]
fn test_document_link_resolve_module() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    // Mark as initialized
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Create a deferred module link (as returned by documentLink)
    let link = json!({
        "range": {
            "start": {"line": 0, "character": 4},
            "end": {"line": 0, "character": 16}
        },
        "tooltip": "Open Data::Dumper",
        "data": {
            "type": "module",
            "module": "Data::Dumper"
        }
    });

    // Resolve the link
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    assert!(resp.result.is_some());

    let result = resp.result.ok_or("Expected result field in response")?;

    // Should have a target now (either local file or MetaCPAN)
    assert!(result.get("target").is_some());
    let target = result["target"].as_str().ok_or("Expected target to be a string")?;

    // Should be either a file:// URI or https://metacpan.org
    assert!(target.starts_with("file://") || target.starts_with("https://metacpan.org"));

    // Data field should be preserved
    assert!(result.get("data").is_some());

    Ok(())
}

/// Test that missing modules fall back to a stable MetaCPAN target
#[test]
fn test_document_link_resolve_missing_module_uses_metacpan_fallback() -> TestResult {
    let server = LspServer::new();

    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    let link = json!({
        "range": {
            "start": {"line": 0, "character": 4},
            "end": {"line": 0, "character": 39}
        },
        "tooltip": "Open Definitely::Missing::Module",
        "data": {
            "type": "module",
            "module": "Definitely::Missing::Module"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let result = resp.result.ok_or("Expected result field in response")?;

    assert_eq!(result["target"], "https://metacpan.org/pod/Definitely::Missing::Module");
    assert!(result.get("data").is_some(), "resolved link should preserve data");

    Ok(())
}

/// Test that documentLink/resolve handles file path links
#[test]
fn test_document_link_resolve_file() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Create a deferred file link
    let link = json!({
        "range": {
            "start": {"line": 0, "character": 9},
            "end": {"line": 0, "character": 20}
        },
        "tooltip": "Open lib/Foo.pm",
        "data": {
            "type": "file",
            "path": "lib/Foo.pm",
            "baseUri": "file:///workspace/script.pl"
        }
    });

    // Resolve the link
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    assert!(resp.result.is_some());

    let result = resp.result.ok_or("Expected result field in response")?;

    // Should have a target now
    assert!(result.get("target").is_some());
    let target = result["target"].as_str().ok_or("Expected target to be a string")?;

    // Should be a file:// URI
    assert!(target.starts_with("file://"));
    assert!(target.contains("lib/Foo.pm") || target.contains("lib\\Foo.pm")); // Windows vs Unix

    Ok(())
}

/// Test that already-resolved links pass through unchanged
#[test]
fn test_document_link_resolve_already_resolved() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Link with target already set
    let link = json!({
        "range": {
            "start": {"line": 0, "character": 4},
            "end": {"line": 0, "character": 16}
        },
        "target": "https://metacpan.org/pod/Data::Dumper",
        "tooltip": "Open Data::Dumper"
    });

    // Resolve should return unchanged
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link.clone()),
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    assert!(resp.result.is_some());

    let result = resp.result.ok_or("Expected result field in response")?;

    // Target should be unchanged
    assert_eq!(result["target"], "https://metacpan.org/pod/Data::Dumper");

    Ok(())
}

/// Test error handling for invalid link data
#[test]
fn test_document_link_resolve_invalid_data() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Link with unknown type
    let link = json!({
        "range": {
            "start": {"line": 0, "character": 4},
            "end": {"line": 0, "character": 16}
        },
        "data": {
            "type": "unknown"
        }
    });

    // Should return error
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;

    // Should be an error response
    assert!(resp.error.is_some());
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602); // INVALID_PARAMS

    Ok(())
}

/// Test error handling for malformed module target data
#[test]
fn test_document_link_resolve_rejects_invalid_module_target() -> TestResult {
    let server = LspServer::new();

    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    let link = json!({
        "range": {
            "start": {"line": 0, "character": 4},
            "end": {"line": 0, "character": 13}
        },
        "data": {
            "type": "module",
            "module": "Local/Bad"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("Invalid module name"),
        "expected invalid module error, got: {}",
        error.message
    );

    Ok(())
}

/// Test error handling for missing parameters
#[test]
fn test_document_link_resolve_missing_params() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // No params
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: None,
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;

    // Should be an error response
    assert!(resp.error.is_some());
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602); // INVALID_PARAMS

    Ok(())
}

/// Test that data field is preserved in resolved link
#[test]
fn test_document_link_resolve_preserves_data() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Link with data
    let link = json!({
        "range": {
            "start": {"line": 0, "character": 4},
            "end": {"line": 0, "character": 16}
        },
        "tooltip": "Open Foo::Bar",
        "data": {
            "type": "module",
            "module": "Foo::Bar",
            "custom": "metadata"
        }
    });

    // Resolve the link
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link.clone()),
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    assert!(resp.result.is_some());

    let result = resp.result.ok_or("Expected result field in response")?;

    // Data field should be preserved
    assert!(result.get("data").is_some());
    let data = &result["data"];
    assert_eq!(data["type"], "module");
    assert_eq!(data["module"], "Foo::Bar");
    assert_eq!(data["custom"], "metadata");

    Ok(())
}

/// Test URL type links (already resolved)
#[test]
fn test_document_link_resolve_url_type() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Link with URL type
    let link = json!({
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": 10}
        },
        "data": {
            "type": "url",
            "url": "https://example.com"
        }
    });

    // Resolve should set target from data.url
    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    assert!(response.is_some());
    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    assert!(resp.result.is_some());

    let result = resp.result.ok_or("Expected result field in response")?;

    // Target should be set from data.url
    assert_eq!(result["target"], "https://example.com");

    Ok(())
}

/// Test POD section links resolve to same-document URI fragments
#[test]
fn test_document_link_resolve_pod_section() -> TestResult {
    let server = LspServer::new();

    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    let link = json!({
        "range": {
            "start": {"line": 2, "character": 6},
            "end": {"line": 2, "character": 18}
        },
        "tooltip": "Open POD section method_name",
        "data": {
            "type": "pod_section",
            "section": "method_name",
            "baseUri": "file:///workspace/test.pl"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let result = resp.result.ok_or("Expected result field in response")?;

    assert_eq!(result["target"], "file:///workspace/test.pl#method_name");
    assert_eq!(data_str(&result, "/data/type"), Some("pod_section"));
    assert_eq!(data_str(&result, "/data/section"), Some("method_name"));

    Ok(())
}

/// Test malformed POD section data is rejected as invalid params
#[test]
fn test_document_link_resolve_rejects_invalid_pod_section() -> TestResult {
    let server = LspServer::new();

    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    let link = json!({
        "range": {
            "start": {"line": 2, "character": 6},
            "end": {"line": 2, "character": 23}
        },
        "data": {
            "type": "pod_section",
            "section": "Other/section",
            "baseUri": "file:///workspace/test.pl"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("Invalid POD section"),
        "expected invalid POD section error, got: {}",
        error.message
    );

    Ok(())
}

/// Test malformed POD section links without section data are rejected
#[test]
fn test_document_link_resolve_rejects_pod_section_missing_section() -> TestResult {
    let server = initialized_server();
    let link = json!({
        "range": {
            "start": {"line": 2, "character": 6},
            "end": {"line": 2, "character": 18}
        },
        "data": {
            "type": "pod_section",
            "baseUri": "file:///workspace/test.pl"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("Missing POD section"),
        "expected missing POD section error, got: {}",
        error.message
    );

    Ok(())
}

/// Test malformed POD section links without base URI data are rejected
#[test]
fn test_document_link_resolve_rejects_pod_section_missing_base_uri() -> TestResult {
    let server = initialized_server();
    let link = json!({
        "range": {
            "start": {"line": 2, "character": 6},
            "end": {"line": 2, "character": 18}
        },
        "data": {
            "type": "pod_section",
            "section": "method_name"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("Missing base URI"),
        "expected missing base URI error, got: {}",
        error.message
    );

    Ok(())
}

/// Test malformed POD section links with invalid base URI data are rejected
#[test]
fn test_document_link_resolve_rejects_pod_section_invalid_base_uri() -> TestResult {
    let server = initialized_server();
    let link = json!({
        "range": {
            "start": {"line": 2, "character": 6},
            "end": {"line": 2, "character": 18}
        },
        "data": {
            "type": "pod_section",
            "section": "method_name",
            "baseUri": "not a uri"
        }
    });

    let response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(link),
    });

    let resp = response.ok_or("Expected response from documentLink/resolve")?;
    let error = resp.error.ok_or("Expected error field in response")?;
    assert_eq!(error.code, -32602);
    assert!(
        error.message.contains("Invalid base URI"),
        "expected invalid base URI error, got: {}",
        error.message
    );

    Ok(())
}

/// Integration test: documentLink returns deferred links, resolve fills them in
#[test]
fn test_document_link_integration() -> TestResult {
    let server = LspServer::new();

    // Initialize server
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    // Open a document with module references
    let doc_uri = "file:///workspace/test.pl";
    let doc_text = r#"use Data::Dumper;
use JSON::XS;
require Foo::Bar;
"#;

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": doc_uri,
                "languageId": "perl",
                "version": 1,
                "text": doc_text
            }
        })),
    });

    // Get document links
    let link_response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "textDocument/documentLink".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": doc_uri
            }
        })),
    });

    assert!(link_response.is_some());
    let link_resp = link_response.ok_or("Expected response from documentLink")?;
    assert!(link_resp.result.is_some());

    let links = link_resp.result.ok_or("Expected result field in link response")?;
    let links_array = links.as_array().ok_or("Expected links to be an array")?;

    // Should have links for Data::Dumper, JSON::XS, and Foo::Bar
    assert!(links_array.len() >= 3);

    // All links should have data field (deferred resolution)
    for link in links_array {
        assert!(link.get("data").is_some());
        assert!(link.get("data").ok_or("Expected data field")?.get("type").is_some());

        // Resolve each link
        let resolve_response = server.handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(perl_lsp::protocol::JsonRpcId::Integer(3_i64)),
            method: "documentLink/resolve".to_string(),
            params: Some(link.clone()),
        });

        assert!(resolve_response.is_some());
        let resolve_resp = resolve_response.ok_or("Expected response from documentLink/resolve")?;
        assert!(resolve_resp.result.is_some());

        let resolved = resolve_resp.result.ok_or("Expected result field in resolve response")?;

        // Should now have a target
        assert!(resolved.get("target").is_some());
    }

    Ok(())
}

/// Integration test: documentLink extracts POD L<> links and resolve fills targets
#[test]
fn test_document_link_integration_pod_l_markup() -> TestResult {
    let server = LspServer::new();

    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///workspace"
    });
    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(1_i64)),
        method: "initialize".to_string(),
        params: Some(init_params),
    });

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    });

    let doc_uri = "file:///workspace/test.pl";
    let doc_text = r#"package Local::Doc;

=head1 SEE ALSO

See L<Data::Dumper>, L<strict>, L</method_name>, L<Local::Doc>, and L<NotAModule>.

=cut

1;
"#;

    let _ = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": doc_uri,
                "languageId": "perl",
                "version": 1,
                "text": doc_text
            }
        })),
    });

    let link_response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(2_i64)),
        method: "textDocument/documentLink".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": doc_uri
            }
        })),
    });

    let link_resp = link_response.ok_or("Expected response from documentLink")?;
    let links = link_resp.result.ok_or("Expected result field in link response")?;
    let links_array = links.as_array().ok_or("Expected links to be an array")?;

    let module_link = links_array
        .iter()
        .find(|link| data_str(link, "/data/module") == Some("Data::Dumper"))
        .ok_or("expected Data::Dumper POD document link")?;
    let pragma_link = links_array
        .iter()
        .find(|link| data_str(link, "/data/module") == Some("strict"))
        .ok_or("expected strict POD document link")?;
    let section_link = links_array
        .iter()
        .find(|link| {
            data_str(link, "/data/type") == Some("pod_section")
                && data_str(link, "/data/section") == Some("method_name")
        })
        .ok_or("expected POD section document link")?;

    assert!(
        !links_array.iter().any(|link| data_str(link, "/data/module") == Some("Local::Doc")),
        "self POD links should stay quiet: {links_array:#?}"
    );
    assert!(
        !links_array.iter().any(|link| data_str(link, "/data/module") == Some("NotAModule")),
        "single-segment unknown POD links should stay quiet: {links_array:#?}"
    );

    for link in [module_link, pragma_link] {
        let resolve_response = server.handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".to_string(),
            id: Some(perl_lsp::protocol::JsonRpcId::Integer(3_i64)),
            method: "documentLink/resolve".to_string(),
            params: Some(link.clone()),
        });

        let resolve_resp = resolve_response.ok_or("Expected response from documentLink/resolve")?;
        let resolved = resolve_resp.result.ok_or("Expected result field in resolve response")?;
        assert!(resolved.get("target").is_some());
    }

    let section_response = server.handle_request(JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(perl_lsp::protocol::JsonRpcId::Integer(4_i64)),
        method: "documentLink/resolve".to_string(),
        params: Some(section_link.clone()),
    });
    let section_resp = section_response.ok_or("Expected response from documentLink/resolve")?;
    let section_resolved = section_resp.result.ok_or("Expected section resolve result")?;
    assert_eq!(section_resolved["target"], "file:///workspace/test.pl#method_name");

    Ok(())
}
