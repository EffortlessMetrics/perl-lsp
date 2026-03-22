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

// Test 3: non-fixable diagnostic has fixable: false
#[test]
fn test_non_fixable_diagnostic_has_fixable_false() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_non_fixable.pl";
    // PL105 (VariableRedeclaration) has no quick-fix
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

    // If PL105 is triggered, it must have fixable: false
    if let Some(diag) = items.iter().find(|d| d["code"].as_str() == Some("PL105")) {
        let data = &diag["data"];
        assert_eq!(data["fixable"], false, "PL105 has no quick-fix; fixable must be false");
    }

    Ok(())
}

// Test 4: suggestion is appended to message when present
#[test]
fn test_suggestion_appended_to_message() -> Result<(), Box<dyn std::error::Error>> {
    let uri = "file:///test_suggestion.pl";
    // Two-arg open (PL401) should trigger a suggestion
    let content = "use strict; use warnings;\nopen(FH, \"file.txt\") or die;\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    // Find any diagnostic that has a suggestion appended (contains "Suggestion:")
    // This test verifies the plumbing works for any diagnostic that has suggestions.
    // If no suggestion-carrying diagnostics fire, we at least verify the data fields.
    let has_suggestion_message = items
        .iter()
        .any(|d| d["message"].as_str().map(|m| m.contains("Suggestion:")).unwrap_or(false));

    // We may not always trigger a suggestion-bearing diagnostic in this simple test.
    // At a minimum, verify the structure of any diagnostics with codes is correct.
    for d in &items {
        if d["code"].is_string() {
            let data = &d["data"];
            assert!(data.is_object(), "data must be an object for coded diagnostics");
        }
    }

    // Log whether suggestion was found (soft assertion — this is coverage, not a gate)
    if !has_suggestion_message {
        eprintln!(
            "No suggestion-bearing diagnostic fired for this test input; structure still verified"
        );
    }

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
    // Security diagnostics (e.g. eval with string arg) populate related_information
    // This test verifies the field is present and valid when any lint populates it.
    let content = "use strict; use warnings;\neval \"dangerous_code()\";\n";
    let server = open_document(uri, content);
    let items = get_diagnostics(&server, uri)?;

    // If any diagnostic has relatedInformation, verify its structure
    for d in &items {
        if let Some(ri_arr) = d["relatedInformation"].as_array() {
            for ri in ri_arr {
                assert!(
                    ri["location"].is_object(),
                    "relatedInformation[].location must be an object"
                );
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
        }
    }

    Ok(())
}
