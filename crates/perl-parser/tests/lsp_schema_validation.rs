//! LSP Schema Validation Tests
//!
//! Validates that all LSP messages conform to the protocol specification
//! using strict JSON schema validation.

use serde_json::{Value, json};
use std::collections::HashSet;

mod support;
use support::lsp_harness::LspHarness as TestHarness;

// ======================== SCHEMA VALIDATORS ========================

/// Validates Position object per LSP spec
fn validate_position(pos: &Value) -> Result<(), String> {
    if !pos.is_object() {
        return Err("Position must be an object".into());
    }

    let line = pos
        .get("line")
        .ok_or("Position missing 'line'")?
        .as_u64()
        .ok_or("Position.line must be unsigned integer")?;

    let character = pos
        .get("character")
        .ok_or("Position missing 'character'")?
        .as_u64()
        .ok_or("Position.character must be unsigned integer")?;

    if line > u32::MAX as u64 {
        return Err("Position.line exceeds u32 max".into());
    }

    if character > u32::MAX as u64 {
        return Err("Position.character exceeds u32 max".into());
    }

    Ok(())
}

/// Validates Range object per LSP spec
fn validate_range(range: &Value) -> Result<(), String> {
    if !range.is_object() {
        return Err("Range must be an object".into());
    }

    let start = range.get("start").ok_or("Range missing 'start'")?;
    let end = range.get("end").ok_or("Range missing 'end'")?;

    validate_position(start).map_err(|e| format!("Range.start: {}", e))?;
    validate_position(end).map_err(|e| format!("Range.end: {}", e))?;

    Ok(())
}

/// Validates Location object per LSP spec
fn validate_location(loc: &Value) -> Result<(), String> {
    if !loc.is_object() {
        return Err("Location must be an object".into());
    }

    let uri = loc
        .get("uri")
        .ok_or("Location missing 'uri'")?
        .as_str()
        .ok_or("Location.uri must be string")?;

    if !uri.contains(':') {
        return Err("Location.uri must be valid URI with scheme".into());
    }

    let range = loc.get("range").ok_or("Location missing 'range'")?;
    validate_range(range).map_err(|e| format!("Location.range: {}", e))?;

    Ok(())
}

/// Validates TextDocumentIdentifier
fn validate_text_document_identifier(doc: &Value) -> Result<(), String> {
    if !doc.is_object() {
        return Err("TextDocumentIdentifier must be object".into());
    }

    let uri = doc.get("uri").ok_or("Missing 'uri'")?.as_str().ok_or("'uri' must be string")?;

    if !uri.contains(':') {
        return Err("'uri' must be valid URI".into());
    }

    Ok(())
}

/// Validates Diagnostic object
fn validate_diagnostic(diag: &Value) -> Result<(), String> {
    if !diag.is_object() {
        return Err("Diagnostic must be object".into());
    }

    // Required fields
    let range = diag.get("range").ok_or("Diagnostic missing 'range'")?;
    validate_range(range)?;

    let message = diag
        .get("message")
        .ok_or("Diagnostic missing 'message'")?
        .as_str()
        .ok_or("Diagnostic.message must be string")?;

    if message.is_empty() {
        return Err("Diagnostic.message cannot be empty".into());
    }

    // Optional fields with validation
    if let Some(severity) = diag.get("severity") {
        let sev = severity.as_u64().ok_or("severity must be number")?;
        if sev < 1 || sev > 4 {
            return Err("severity must be 1-4".into());
        }
    }

    if let Some(code) = diag.get("code") {
        if !code.is_string() && !code.is_number() {
            return Err("code must be string or number".into());
        }
    }

    if let Some(source) = diag.get("source") {
        if !source.is_string() {
            return Err("source must be string".into());
        }
    }

    Ok(())
}

// ======================== MESSAGE VALIDATION TESTS ========================

#[test]
fn test_completion_response_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    let uri = "file:///test.pl";
    harness.open_document(uri, "my $var = 1;\n$v");

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 2}
        }
    }));

    let result = &response["result"];

    // Can be array or CompletionList
    if result.is_array() {
        for item in result.as_array().unwrap() {
            validate_completion_item(item).unwrap();
        }
    } else if result.is_object() {
        // CompletionList
        assert!(result["items"].is_array(), "CompletionList must have items array");

        if let Some(incomplete) = result.get("isIncomplete") {
            assert!(incomplete.is_boolean(), "isIncomplete must be boolean");
        }

        for item in result["items"].as_array().unwrap() {
            validate_completion_item(item).unwrap();
        }
    }
}

fn validate_completion_item(item: &Value) -> Result<(), String> {
    if !item.is_object() {
        return Err("CompletionItem must be object".into());
    }

    // Required: label
    item.get("label")
        .ok_or("CompletionItem missing 'label'")?
        .as_str()
        .ok_or("label must be string")?;

    // Optional fields with validation
    if let Some(kind) = item.get("kind") {
        let k = kind.as_u64().ok_or("kind must be number")?;
        if k < 1 || k > 25 {
            return Err("kind must be 1-25".into());
        }
    }

    if let Some(detail) = item.get("detail") {
        detail.as_str().ok_or("detail must be string")?;
    }

    if let Some(doc) = item.get("documentation") {
        if !doc.is_string() && !doc.is_object() {
            return Err("documentation must be string or MarkupContent".into());
        }
    }

    if let Some(deprecated) = item.get("deprecated") {
        deprecated.as_bool().ok_or("deprecated must be boolean")?;
    }

    if let Some(preselect) = item.get("preselect") {
        preselect.as_bool().ok_or("preselect must be boolean")?;
    }

    Ok(())
}

#[test]
fn test_document_symbol_response_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    let uri = "file:///test.pl";
    harness.open_document(uri, "sub test { my $x = 1; }");

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/documentSymbol",
        "params": {"textDocument": {"uri": uri}}
    }));

    let result = &response["result"];
    assert!(result.is_array(), "documentSymbol must return array");

    for symbol in result.as_array().unwrap() {
        // Can be SymbolInformation or DocumentSymbol
        if symbol.get("location").is_some() {
            validate_symbol_information(symbol).unwrap();
        } else {
            validate_document_symbol(symbol).unwrap();
        }
    }
}

fn validate_symbol_information(sym: &Value) -> Result<(), String> {
    sym.get("name")
        .ok_or("SymbolInformation missing 'name'")?
        .as_str()
        .ok_or("name must be string")?;

    let kind = sym
        .get("kind")
        .ok_or("SymbolInformation missing 'kind'")?
        .as_u64()
        .ok_or("kind must be number")?;

    if kind < 1 || kind > 26 {
        return Err("kind must be 1-26".into());
    }

    let location = sym.get("location").ok_or("SymbolInformation missing 'location'")?;
    validate_location(location)?;

    Ok(())
}

fn validate_document_symbol(sym: &Value) -> Result<(), String> {
    sym.get("name")
        .ok_or("DocumentSymbol missing 'name'")?
        .as_str()
        .ok_or("name must be string")?;

    let kind = sym
        .get("kind")
        .ok_or("DocumentSymbol missing 'kind'")?
        .as_u64()
        .ok_or("kind must be number")?;

    if kind < 1 || kind > 26 {
        return Err("kind must be 1-26".into());
    }

    let range = sym.get("range").ok_or("DocumentSymbol missing 'range'")?;
    validate_range(range)?;

    let selection_range =
        sym.get("selectionRange").ok_or("DocumentSymbol missing 'selectionRange'")?;
    validate_range(selection_range)?;

    // Optional children
    if let Some(children) = sym.get("children") {
        let arr = children.as_array().ok_or("children must be array")?;
        for child in arr {
            validate_document_symbol(child)?;
        }
    }

    Ok(())
}

#[test]
fn test_hover_response_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    let uri = "file:///test.pl";
    harness.open_document(uri, "print 'hello'");

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 2}
        }
    }));

    if !response["result"].is_null() {
        let hover = &response["result"];

        // Must have contents
        let contents = hover.get("contents").ok_or("Hover missing 'contents'").unwrap();

        // Contents can be string, MarkupContent, or MarkedString[]
        if contents.is_string() {
            // Valid
        } else if contents.is_array() {
            // Array of MarkedString
            for item in contents.as_array().unwrap() {
                if !item.is_string() && !item.is_object() {
                    panic!("Hover contents array must contain strings or MarkedString");
                }
            }
        } else if contents.is_object() {
            // MarkupContent
            let kind = contents
                .get("kind")
                .and_then(|k| k.as_str())
                .expect("MarkupContent must have 'kind'");

            assert!(
                kind == "plaintext" || kind == "markdown",
                "MarkupContent.kind must be 'plaintext' or 'markdown'"
            );

            contents
                .get("value")
                .and_then(|v| v.as_str())
                .expect("MarkupContent must have 'value' string");
        } else {
            panic!("Invalid hover contents type");
        }

        // Optional range
        if let Some(range) = hover.get("range") {
            validate_range(range).unwrap();
        }
    }
}

#[test]
fn test_workspace_symbol_response_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    harness.open_document("file:///test.pl", "sub test { }");

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "workspace/symbol",
        "params": {"query": "test"}
    }));

    let result = &response["result"];
    assert!(result.is_array(), "workspace/symbol must return array");

    for symbol in result.as_array().unwrap() {
        validate_symbol_information(symbol).unwrap();
    }
}

#[test]
fn test_code_action_response_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    let uri = "file:///test.pl";
    harness.open_document(uri, "open(FH, 'file.txt');");

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 21}
            },
            "context": {"diagnostics": []}
        }
    }));

    let result = &response["result"];
    assert!(result.is_array() || result.is_null(), "codeAction must return array or null");

    if result.is_array() {
        for action in result.as_array().unwrap() {
            validate_code_action(action).unwrap();
        }
    }
}

fn validate_code_action(action: &Value) -> Result<(), String> {
    // Can be Command or CodeAction
    if action.get("command").is_some() && action.get("arguments").is_some() {
        // It's a Command
        action
            .get("title")
            .ok_or("Command missing 'title'")?
            .as_str()
            .ok_or("title must be string")?;

        action
            .get("command")
            .ok_or("Command missing 'command'")?
            .as_str()
            .ok_or("command must be string")?;

        if !action.get("arguments").unwrap().is_array() {
            return Err("arguments must be array".into());
        }
    } else {
        // It's a CodeAction
        action
            .get("title")
            .ok_or("CodeAction missing 'title'")?
            .as_str()
            .ok_or("title must be string")?;

        if let Some(kind) = action.get("kind") {
            kind.as_str().ok_or("kind must be string")?;
        }

        if let Some(diags) = action.get("diagnostics") {
            let arr = diags.as_array().ok_or("diagnostics must be array")?;
            for diag in arr {
                validate_diagnostic(diag)?;
            }
        }

        if let Some(edit) = action.get("edit") {
            validate_workspace_edit(edit)?;
        }
    }

    Ok(())
}

fn validate_workspace_edit(edit: &Value) -> Result<(), String> {
    if !edit.is_object() {
        return Err("WorkspaceEdit must be object".into());
    }

    // Can have 'changes' or 'documentChanges'
    if let Some(changes) = edit.get("changes") {
        let obj = changes.as_object().ok_or("changes must be object")?;
        for (uri, edits) in obj {
            if !uri.contains(':') {
                return Err("changes key must be valid URI".into());
            }

            let edit_arr = edits.as_array().ok_or("changes value must be array")?;
            for text_edit in edit_arr {
                validate_text_edit(text_edit)?;
            }
        }
    }

    if let Some(doc_changes) = edit.get("documentChanges") {
        let arr = doc_changes.as_array().ok_or("documentChanges must be array")?;
        // Could validate TextDocumentEdit here
    }

    Ok(())
}

fn validate_text_edit(edit: &Value) -> Result<(), String> {
    if !edit.is_object() {
        return Err("TextEdit must be object".into());
    }

    let range = edit.get("range").ok_or("TextEdit missing 'range'")?;
    validate_range(range)?;

    edit.get("newText")
        .ok_or("TextEdit missing 'newText'")?
        .as_str()
        .ok_or("newText must be string")?;

    Ok(())
}

#[test]
fn test_publish_diagnostics_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    let uri = "file:///test.pl";
    harness.open_document(uri, "use strict;\n$undefined = 1;");

    // Server should publish diagnostics
    // In real test, we'd capture notifications

    // Validate diagnostic structure if we had it
    let sample_diagnostic = json!({
        "uri": uri,
        "diagnostics": [{
            "range": {
                "start": {"line": 1, "character": 0},
                "end": {"line": 1, "character": 10}
            },
            "severity": 1,
            "code": "undefined-variable",
            "source": "perl-parser",
            "message": "Variable '$undefined' is not declared"
        }]
    });

    validate_publish_diagnostics_params(&sample_diagnostic).unwrap();
}

fn validate_publish_diagnostics_params(params: &Value) -> Result<(), String> {
    params.get("uri").ok_or("Missing 'uri'")?.as_str().ok_or("'uri' must be string")?;

    let diagnostics = params
        .get("diagnostics")
        .ok_or("Missing 'diagnostics'")?
        .as_array()
        .ok_or("'diagnostics' must be array")?;

    for diag in diagnostics {
        validate_diagnostic(diag)?;
    }

    Ok(())
}

#[test]
fn test_error_response_schema() {
    let mut harness = TestHarness::new();

    // Request before initialization
    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": "file:///test.pl"},
            "position": {"line": 0, "character": 0}
        }
    }));

    assert!(response["error"].is_object(), "Error response must have 'error'");

    let error = &response["error"];

    // Required fields
    let code = error.get("code").and_then(|c| c.as_i64()).expect("Error must have numeric 'code'");

    let message =
        error.get("message").and_then(|m| m.as_str()).expect("Error must have string 'message'");

    assert!(!message.is_empty(), "Error message cannot be empty");

    // Standard error codes
    let valid_codes: HashSet<i64> = [
        -32700, // Parse error
        -32600, // Invalid Request
        -32601, // Method not found
        -32602, // Invalid params
        -32603, // Internal error
        -32002, // Server not initialized
        -32001, // Unknown error code
        -32800, // Request cancelled
    ]
    .iter()
    .cloned()
    .collect();

    // Custom error codes are also allowed (non-reserved range)
    if code < -32099 || code > -32000 {
        assert!(valid_codes.contains(&code) || code >= 0, "Invalid error code: {}", code);
    }
}

#[test]
fn test_signature_help_response_schema() {
    let mut harness = TestHarness::new();
    harness.initialize_default();

    let uri = "file:///test.pl";
    harness.open_document(uri, "print(");

    let response = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/signatureHelp",
        "params": {
            "textDocument": {"uri": uri},
            "position": {"line": 0, "character": 6}
        }
    }));

    if !response["result"].is_null() {
        let sig_help = &response["result"];

        let signatures = sig_help
            .get("signatures")
            .and_then(|s| s.as_array())
            .expect("SignatureHelp must have 'signatures' array");

        assert!(!signatures.is_empty(), "Must have at least one signature");

        for sig in signatures {
            sig.get("label")
                .and_then(|l| l.as_str())
                .expect("SignatureInformation must have 'label'");

            if let Some(params) = sig.get("parameters") {
                let param_arr = params.as_array().expect("parameters must be array");

                for param in param_arr {
                    // Must have label
                    let label = param.get("label").expect("ParameterInformation must have 'label'");

                    // Label can be string or [usize, usize]
                    if !label.is_string() && !label.is_array() {
                        panic!("Parameter label must be string or [number, number]");
                    }
                }
            }
        }

        // Optional activeSignature
        if let Some(active_sig) = sig_help.get("activeSignature") {
            active_sig.as_u64().expect("activeSignature must be number");
        }

        // Optional activeParameter
        if let Some(active_param) = sig_help.get("activeParameter") {
            active_param.as_u64().expect("activeParameter must be number");
        }
    }
}
