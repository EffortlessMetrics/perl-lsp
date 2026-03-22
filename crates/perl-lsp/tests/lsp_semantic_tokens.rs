use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

#[test]
fn semantic_tokens_emit_data() -> Result<(), Box<dyn std::error::Error>> {
    let srv = LspServer::new();
    let init = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "initialize".into(),
        params: Some(json!({"capabilities":{}})),
    };
    srv.handle_request(init);

    // Send initialized notification (required by LSP protocol)
    let initialized = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "initialized".into(),
        params: Some(json!({})),
    };
    srv.handle_request(initialized);

    let uri = "file:///tokens.pl";
    let text = r#"package Foo; my $x = 1; sub bar { return $x } $x = 2; bar();"#;
    let open = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: None,
        method: "textDocument/didOpen".into(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    srv.handle_request(open);

    let req = JsonRpcRequest {
        _jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "textDocument/semanticTokens/full".into(),
        params: Some(json!({"textDocument": {"uri": uri}})),
    };
    let res = srv.handle_request(req).ok_or("handle_request returned None")?;
    let result = res.result.ok_or("response result is None")?;
    let arr = result["data"].as_array().ok_or("data field is not an array")?;
    assert!(!arr.is_empty(), "semantic tokens should return data");

    // Verify encoding is valid (5-tuples)
    assert_eq!(arr.len() % 5, 0, "semantic tokens must be 5-tuples");

    Ok(())
}

// =========================================================================
// Semantic tokens delta encoding tests (issue #2320)
// =========================================================================

mod support;
use support::lsp_harness::LspHarness;

/// Full request must include a `resultId` field for delta tracking.
#[test]
fn semantic_tokens_full_returns_result_id() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    let uri = "file:///delta_test.pl";
    harness.open(uri, "package Foo;\nsub bar { my $x = 1; }")?;

    let result = harness
        .request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": uri } }))
        .map_err(|e| e)?;

    assert!(
        result.get("resultId").is_some(),
        "full response must contain resultId for delta tracking, got: {}",
        result
    );
    assert!(result["data"].is_array(), "full response must contain data array");
    Ok(())
}

/// After an edit, the delta request must return an `edits` array (delta response).
#[test]
fn semantic_tokens_delta_returns_edits_after_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    let uri = "file:///delta_edit_test.pl";
    harness.open(uri, "package Foo;\nsub bar { my $x = 1; }")?;

    // Get initial full tokens
    let full_result = harness
        .request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": uri } }))
        .map_err(|e| e)?;

    let result_id =
        full_result["resultId"].as_str().ok_or("full response missing resultId")?.to_string();

    // Edit the document (add a new variable)
    harness.change_full(uri, 2, "package Foo;\nsub bar { my $x = 1; my $y = 2; }")?;

    // Request delta
    let delta_result = harness
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri },
                "previousResultId": result_id
            }),
        )
        .map_err(|e| e)?;

    // Delta response must have `edits` array (not `data`)
    assert!(
        delta_result.get("edits").is_some(),
        "delta response must contain edits array, got: {}",
        delta_result
    );
    assert!(delta_result.get("resultId").is_some(), "delta response must contain updated resultId");
    Ok(())
}

/// No-op edit: tokens unchanged, delta returns empty edits array.
#[test]
fn semantic_tokens_delta_noop_returns_empty_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    let uri = "file:///delta_noop_test.pl";
    let content = "package Foo;\nsub bar { my $x = 1; }";
    harness.open(uri, content)?;

    // Get initial full tokens
    let full_result = harness
        .request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": uri } }))
        .map_err(|e| e)?;

    let result_id =
        full_result["resultId"].as_str().ok_or("full response missing resultId")?.to_string();

    // Request delta without any document change
    let delta_result = harness
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri },
                "previousResultId": result_id
            }),
        )
        .map_err(|e| e)?;

    let edits = delta_result["edits"]
        .as_array()
        .ok_or_else(|| format!("expected edits array, got: {}", delta_result))?;
    assert!(
        edits.is_empty(),
        "no-op delta must return empty edits array, got {} edits",
        edits.len()
    );
    Ok(())
}

/// Stale resultId: server returns full tokens (not delta).
#[test]
fn semantic_tokens_delta_stale_id_returns_full() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    let uri = "file:///delta_stale_test.pl";
    harness.open(uri, "package Foo;\nsub bar { my $x = 1; }")?;

    // Request delta with bogus previousResultId (simulates stale client state)
    let delta_result = harness
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri },
                "previousResultId": "nonexistent-result-id-12345"
            }),
        )
        .map_err(|e| e)?;

    // Must return full data, not a delta
    assert!(
        delta_result.get("data").is_some(),
        "stale resultId must fall back to full response with data field, got: {}",
        delta_result
    );
    assert!(delta_result.get("resultId").is_some(), "fallback full response must include resultId");
    Ok(())
}

/// No prior full request: delta with no cache falls back to full response.
#[test]
fn semantic_tokens_delta_no_previous_returns_full() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    let uri = "file:///delta_no_prior_test.pl";
    harness.open(uri, "package Foo;\nsub bar { my $x = 1; }")?;

    // Send delta request without any prior full request
    let delta_result = harness
        .request(
            "textDocument/semanticTokens/full/delta",
            json!({
                "textDocument": { "uri": uri },
                "previousResultId": "no-prior-full-request"
            }),
        )
        .map_err(|e| e)?;

    // Must return full data array since no cache exists
    assert!(
        delta_result.get("data").is_some(),
        "delta with no prior cache must fall back to full response with data, got: {}",
        delta_result
    );
    Ok(())
}

/// Initialize response must advertise delta capability.
#[test]
fn semantic_tokens_capability_advertises_delta() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let caps = &init_response["capabilities"];
    let tokens_provider = &caps["semanticTokensProvider"];

    assert!(!tokens_provider.is_null(), "server must advertise semanticTokensProvider capability");

    // The `full` field must be an object with `delta: true` (not simply Bool(true))
    let full = &tokens_provider["full"];
    assert!(!full.is_null(), "semanticTokensProvider.full must be present");

    let delta = full.get("delta");
    assert!(
        delta.map(|v| v.as_bool()) == Some(Some(true)),
        "semanticTokensProvider.full.delta must be true, got: {}",
        full
    );

    Ok(())
}
