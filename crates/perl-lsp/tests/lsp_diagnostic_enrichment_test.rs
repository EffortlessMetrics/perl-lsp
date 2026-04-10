//! Tests for diagnostic enrichment in the runtime JSON-RPC path (handle_document_diagnostic
//! and handle_workspace_diagnostic). These tests verify that relatedInformation, data
//! (code/category/fixable/tags), and suggestion fields are serialized through the
//! pull-diagnostics runtime path — not just the dead-code PullDiagnosticsProvider path.

use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;

/// Initialize an LspServer and open a document. Returns the server.
fn open_document(uri: &str, content: &str) -> LspServer {
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

    server
}

/// Call textDocument/diagnostic and return the items array.
fn get_diagnostics(
    server: &LspServer,
    uri: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let response = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(json!(99)),
            method: "textDocument/diagnostic".into(),
            params: Some(json!({
                "textDocument": { "uri": uri }
            })),
        })
        .ok_or("No response from textDocument/diagnostic")?;

    let result = response.result.ok_or("Response missing result")?;
    let items = result["items"].as_array().ok_or("Expected items array")?.clone();
    Ok(items)
}

// Test 1: data field populated through JSON-RPC path for a diagnostic with a code
#[test]
fn test_data_field_populated_via_jsonrpc_path() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_data_enrichment.pl";
    // Missing 'use strict' triggers PL100 (MissingStrict)
    let content = "print 'hello';\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    // Find a diagnostic with a code
    let diag_with_code = items
        .iter()
        .find(|d| d["code"].is_string() && d["code"].as_str().unwrap_or("").starts_with("PL"))
        .ok_or("Expected at least one PL-coded diagnostic")?;

    let data = &diag_with_code["data"];
    assert!(data.is_object(), "data must be a JSON object when code is present");
    assert!(data["code"].is_string(), "data.code must be a string");
    assert!(data["category"].is_string(), "data.category must be a string");
    assert!(data["fixable"].is_boolean(), "data.fixable must be a boolean");
    assert!(data["tags"].is_array(), "data.tags must be an array");

    Ok(())
}

// Test 2: PL100 (MissingStrict) data fields have correct values
#[test]
fn test_pl100_data_fields_correct() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_pl100_data.pl";
    let content = "print 'hello';\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    let diag = items
        .iter()
        .find(|d| d["code"].as_str() == Some("PL100"))
        .ok_or("Expected PL100 (MissingStrict) diagnostic")?;

    let data = &diag["data"];
    assert_eq!(data["code"], "PL100", "data.code must match the diagnostic code");
    assert_eq!(
        data["category"], "StrictWarnings",
        "data.category must be StrictWarnings for PL100"
    );
    assert_eq!(data["fixable"], true, "PL100 is fixable (add 'use strict')");

    Ok(())
}

// Test 3: PL105 fixable flag reflects quick-fix availability
#[test]
fn test_pl105_has_fixable_true_after_adding_quick_fix() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_pl105_fixable.pl";
    // PL105 (VariableRedeclaration) now has a quick-fix: remove duplicate 'my' (#3469)
    let content = "use strict; use warnings; my $x = 1; my $x = 2;\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    // Verify every diagnostic with a code has a valid data object
    for d in &items {
        if d["code"].is_string() {
            let data = &d["data"];
            assert!(data.is_object(), "data must be an object when code is present, got: {}", data);
            assert!(data["fixable"].is_boolean(), "data.fixable must be a boolean");
        }
    }

    // PL105 (VariableRedeclaration) MUST fire and now has fixable: true since the
    // "remove duplicate my" quick-fix was added in issue #3469.
    let diag = items
        .iter()
        .find(|d| d["code"].as_str() == Some("PL105"))
        .ok_or("Expected PL105 (VariableRedeclaration) to fire for double-declare input")?;
    let data = &diag["data"];
    assert_eq!(data["fixable"], true, "PL105 now has a quick-fix (remove duplicate 'my')");

    Ok(())
}

// Test 4: suggestion is appended to message when present
#[test]
fn test_suggestion_appended_to_message() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_suggestion.pl";
    // Missing strict/warnings triggers PL100/PL101 which both carry suggestions
    // ("Add 'use strict;' at the top of the file"). This reliably exercises
    // the suggestion serialization path in handle_document_diagnostic.
    let content = "print 'hello';\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    // PL100 must fire and its message must contain the appended suggestion text.
    let pl100 = items
        .iter()
        .find(|d| d["code"].as_str() == Some("PL100"))
        .ok_or("Expected PL100 (MissingStrict) diagnostic to fire")?;

    let msg = pl100["message"].as_str().ok_or("message must be a string")?;
    assert!(
        msg.contains("Suggestion:"),
        "PL100 message must include appended suggestion text; got: {:?}",
        msg
    );

    Ok(())
}

// Test 5: workspace/diagnostic also has data fields populated
#[test]
fn test_workspace_diagnostic_data_populated() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_ws_enrichment.pl";
    // Missing strict/warnings will trigger coded diagnostics
    let content = "print 'hello';\n";
    let server = open_document(uri, content);

    let response = server
        .handle_request(JsonRpcRequest {
            _jsonrpc: "2.0".into(),
            id: Some(json!(99)),
            method: "workspace/diagnostic".into(),
            params: Some(json!({})),
        })
        .ok_or("No response from workspace/diagnostic")?;

    let result = response.result.ok_or("workspace/diagnostic response missing result")?;
    let reports = result["items"].as_array().ok_or("Expected items array")?;

    // Find the full-kind report for our URI
    let our_report = reports
        .iter()
        .find(|r| r["uri"].as_str() == Some(uri) && r["kind"] == "full")
        .ok_or("Expected a full-kind report for our document")?;

    let diag_items = our_report["items"].as_array().ok_or("Expected items in full report")?;

    // Verify every coded diagnostic in the workspace report has data populated
    for d in diag_items {
        if d["code"].is_string() {
            let data = &d["data"];
            assert!(
                data.is_object(),
                "workspace/diagnostic: data must be an object for coded diagnostic, got: {}",
                d
            );
            assert!(data["code"].is_string(), "data.code must be a string");
            assert!(data["category"].is_string(), "data.category must be a string");
            assert!(data["fixable"].is_boolean(), "data.fixable must be a boolean");
            assert!(data["tags"].is_array(), "data.tags must be an array");
        }
    }

    Ok(())
}

// Test 6: relatedInformation forwarded when present in internal diagnostic
#[test]
fn test_related_information_forwarded() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_related_info.pl";
    // eval "string" fires PL600 (SecurityStringEval) which always populates
    // related_information with one entry pointing to the eval location.
    let content = "use strict; use warnings;\neval \"dangerous_code()\";\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    // PL600 must fire.
    let eval_diag = items
        .iter()
        .find(|d| d["code"].as_str() == Some("PL600"))
        .ok_or("Expected PL600 (SecurityStringEval) diagnostic to fire for string eval")?;

    // relatedInformation must be present and non-empty — the internal
    // Diagnostic for PL600 always carries one RelatedInformation entry.
    let ri_arr = eval_diag["relatedInformation"]
        .as_array()
        .ok_or("PL600 must have relatedInformation array in LSP response")?;

    assert!(!ri_arr.is_empty(), "PL600 relatedInformation must be non-empty");

    for ri in ri_arr {
        assert!(ri["location"].is_object(), "relatedInformation[].location must be an object");
        assert!(
            ri["location"]["uri"].is_string(),
            "relatedInformation[].location.uri must be a string"
        );
        assert!(
            ri["location"]["range"].is_object(),
            "relatedInformation[].location.range must be an object"
        );
        assert!(ri["message"].is_string(), "relatedInformation[].message must be a string");
    }

    Ok(())
}
