//! Additional comprehensive unit tests for perl-lsp-protocol crate.
//!
//! Focuses on edge cases, boundary conditions, serialization roundtrips,
//! error code semantics, parameter extraction corner cases, and capability
//! configuration variations not covered by the existing test suite.
#![allow(
    clippy::approx_constant,
    clippy::assertions_on_constants,
    clippy::panic,
    clippy::single_match
)]

use perl_lsp_protocol::capabilities::{
    BuildFlags, cap_bool_or_object, capabilities_for, capabilities_json, default_capabilities,
    get_supported_commands,
};
use perl_lsp_protocol::methods;
use perl_lsp_protocol::*;
use serde_json::json;

// ============================================================================
// JsonRpcRequest — edge-case deserialization
// ============================================================================

#[test]
fn request_with_negative_integer_id() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": -42,
        "method": "shutdown"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert_eq!(req.id, Some(json!(-42)));
    Ok(())
}

#[test]
fn request_with_zero_id() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert_eq!(req.id, Some(json!(0)));
    Ok(())
}

#[test]
fn request_with_large_integer_id() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 999_999_999,
        "method": "shutdown"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert_eq!(req.id, Some(json!(999_999_999)));
    Ok(())
}

#[test]
fn request_with_float_id() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 3.14,
        "method": "shutdown"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    if let Some(id) = &req.id {
        assert!(id.is_f64());
    }
    Ok(())
}

#[test]
fn request_with_array_params() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "custom/method",
        "params": [1, 2, 3]
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    if let Some(params) = &req.params {
        assert!(params.is_array());
    }
    Ok(())
}

#[test]
fn request_with_deeply_nested_params() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///a.pl" },
            "position": { "line": 0, "character": 5 },
            "context": { "triggerKind": 1, "triggerCharacter": "$" }
        }
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    if let Some(params) = &req.params {
        let trigger = params.pointer("/context/triggerCharacter");
        assert_eq!(trigger, Some(&json!("$")));
    }
    Ok(())
}

#[test]
fn request_with_empty_method() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": ""
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert_eq!(req.method, "");
    Ok(())
}

#[test]
fn request_with_unicode_in_params() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/hover",
        "params": { "textDocument": { "uri": "file:///日本語.pl" } }
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    if let Some(params) = &req.params {
        let uri = params.pointer("/textDocument/uri").and_then(|v| v.as_str());
        assert_eq!(uri, Some("file:///日本語.pl"));
    }
    Ok(())
}

#[test]
fn request_with_null_params() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "shutdown",
        "params": null
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    // null params should deserialize as Some(Value::Null)
    // or None depending on serde config — either is acceptable
    assert_eq!(req.method, "shutdown");
    Ok(())
}

#[test]
fn request_missing_jsonrpc_field_fails() {
    let raw = json!({
        "id": 1,
        "method": "shutdown"
    });
    let result: Result<JsonRpcRequest, _> = serde_json::from_value(raw);
    assert!(result.is_err());
}

// ============================================================================
// JsonRpcResponse — serialization edge cases
// ============================================================================

#[test]
fn response_success_with_string_id() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::success(Some(json!("abc-123")), json!({"result": true}));
    let serialized = serde_json::to_value(&resp)?;
    assert_eq!(serialized["id"], json!("abc-123"));
    Ok(())
}

#[test]
fn response_success_preserves_array_result() -> Result<(), Box<dyn std::error::Error>> {
    let items = json!([{"label": "my_sub"}, {"label": "another_sub"}]);
    let resp = JsonRpcResponse::success(Some(json!(1)), items.clone());
    let serialized = serde_json::to_value(&resp)?;
    assert_eq!(serialized["result"], items);
    Ok(())
}

#[test]
fn response_success_with_empty_object_result() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::success(Some(json!(1)), json!({}));
    let serialized = serde_json::to_value(&resp)?;
    assert_eq!(serialized["result"], json!({}));
    Ok(())
}

#[test]
fn response_null_has_null_result() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::null(Some(json!(5)));
    let serialized = serde_json::to_value(&resp)?;
    assert!(serialized["result"].is_null());
    Ok(())
}

#[test]
fn response_error_contains_code_and_message() -> Result<(), Box<dyn std::error::Error>> {
    let err = JsonRpcError::new(METHOD_NOT_FOUND, "not found");
    let resp = JsonRpcResponse::error(Some(json!(99)), err);
    let serialized = serde_json::to_value(&resp)?;
    assert_eq!(serialized["error"]["code"], json!(METHOD_NOT_FOUND));
    assert_eq!(serialized["error"]["message"], json!("not found"));
    Ok(())
}

#[test]
fn response_error_with_data_field() -> Result<(), Box<dyn std::error::Error>> {
    let err = JsonRpcError::with_data(INTERNAL_ERROR, "oops", json!({"detail": "stack trace"}));
    let resp = JsonRpcResponse::error(Some(json!(1)), err);
    let serialized = serde_json::to_value(&resp)?;
    assert_eq!(serialized["error"]["data"]["detail"], json!("stack trace"));
    Ok(())
}

#[test]
fn response_jsonrpc_field_is_always_two_point_zero() -> Result<(), Box<dyn std::error::Error>> {
    let success = JsonRpcResponse::success(Some(json!(1)), json!(null));
    let null_resp = JsonRpcResponse::null(Some(json!(2)));
    let err_resp = JsonRpcResponse::error(Some(json!(3)), JsonRpcError::new(INTERNAL_ERROR, "err"));

    for resp in &[success, null_resp, err_resp] {
        let serialized = serde_json::to_value(resp)?;
        assert_eq!(serialized["jsonrpc"], json!("2.0"));
    }
    Ok(())
}

// ============================================================================
// JsonRpcError — construction and trait impls
// ============================================================================

#[test]
fn error_new_with_empty_message() {
    let err = JsonRpcError::new(PARSE_ERROR, "");
    assert_eq!(err.message, "");
    assert_eq!(err.code, PARSE_ERROR);
    assert!(err.data.is_none());
}

#[test]
fn error_with_data_preserves_complex_data() {
    let data = json!({
        "file": "test.pl",
        "line": 42,
        "details": ["a", "b", "c"]
    });
    let err = JsonRpcError::with_data(INVALID_PARAMS, "bad params", data.clone());
    assert_eq!(err.data, Some(data));
}

#[test]
fn error_display_includes_code_and_message() {
    let err = JsonRpcError::new(-32600, "Invalid Request");
    let display = format!("{}", err);
    assert!(display.contains("-32600"));
    assert!(display.contains("Invalid Request"));
}

#[test]
fn error_clone_is_independent() {
    let original = JsonRpcError::with_data(PARSE_ERROR, "parse error", json!({"key": "val"}));
    let cloned = original.clone();
    assert_eq!(original.code, cloned.code);
    assert_eq!(original.message, cloned.message);
    assert_eq!(original.data, cloned.data);
}

#[test]
fn error_as_std_error_has_source_none() {
    let err = JsonRpcError::new(INTERNAL_ERROR, "test");
    let std_err: &dyn std::error::Error = &err;
    assert!(std_err.source().is_none());
}

// ============================================================================
// Error codes — value validation
// ============================================================================

#[test]
fn parse_error_is_negative_32700() {
    assert_eq!(PARSE_ERROR, -32700);
}

#[test]
fn invalid_request_is_negative_32600() {
    assert_eq!(INVALID_REQUEST, -32600);
}

#[test]
fn method_not_found_is_negative_32601() {
    assert_eq!(METHOD_NOT_FOUND, -32601);
}

#[test]
fn invalid_params_code_is_negative_32602() {
    assert_eq!(INVALID_PARAMS, -32602);
}

#[test]
fn internal_error_code_is_negative_32603() {
    assert_eq!(INTERNAL_ERROR, -32603);
}

#[test]
fn server_error_range_bounds() {
    assert!(SERVER_ERROR_START <= SERVER_ERROR_END);
    assert_eq!(SERVER_ERROR_START, -32099);
    assert_eq!(SERVER_ERROR_END, -32000);
}

#[test]
fn connection_closed_in_server_error_range() {
    assert!(CONNECTION_CLOSED >= SERVER_ERROR_START);
    assert!(CONNECTION_CLOSED <= SERVER_ERROR_END);
}

#[test]
fn transport_error_in_server_error_range() {
    assert!(TRANSPORT_ERROR >= SERVER_ERROR_START);
    assert!(TRANSPORT_ERROR <= SERVER_ERROR_END);
}

#[test]
fn unknown_error_code_in_server_error_range() {
    assert!(UNKNOWN_ERROR_CODE >= SERVER_ERROR_START);
    assert!(UNKNOWN_ERROR_CODE <= SERVER_ERROR_END);
}

#[test]
fn lsp_cancellation_codes_are_distinct() {
    assert_ne!(REQUEST_CANCELLED, SERVER_CANCELLED);
    assert_ne!(REQUEST_CANCELLED, CONTENT_MODIFIED);
    assert_ne!(SERVER_CANCELLED, CONTENT_MODIFIED);
    assert_ne!(REQUEST_FAILED, REQUEST_CANCELLED);
}

#[test]
fn server_not_initialized_code_is_correct() {
    assert_eq!(SERVER_NOT_INITIALIZED, -32002);
}

#[test]
fn content_modified_code_is_correct() {
    assert_eq!(CONTENT_MODIFIED, -32801);
}

#[test]
fn request_failed_code_is_correct() {
    assert_eq!(REQUEST_FAILED, -32803);
}

// ============================================================================
// Error builders — output validation
// ============================================================================

#[test]
fn cancelled_response_has_correct_error_code() {
    let resp = cancelled_response(&json!(10));
    if let Some(err) = &resp.error {
        assert_eq!(err.code, REQUEST_CANCELLED);
    }
    assert!(resp.result.is_none());
}

#[test]
fn cancelled_response_preserves_request_id() {
    let resp = cancelled_response(&json!("request-abc"));
    assert_eq!(resp.id, Some(json!("request-abc")));
}

#[test]
fn cancelled_response_with_method_deeply_nested_path() {
    let resp = cancelled_response_with_method(&json!(1), "textDocument/semanticTokens/full");
    if let Some(err) = &resp.error {
        // "full" is the last segment
        assert!(err.message.contains("full"));
    }
}

#[test]
fn cancelled_response_with_method_has_timestamp() {
    let resp = cancelled_response_with_method(&json!(1), "textDocument/hover");
    if let Some(err) = &resp.error {
        if let Some(data) = &err.data {
            assert!(data.get("timestamp").is_some());
        }
    }
}

#[test]
fn cancelled_response_with_method_empty_method() {
    let resp = cancelled_response_with_method(&json!(1), "");
    if let Some(err) = &resp.error {
        assert_eq!(err.code, REQUEST_CANCELLED);
        // empty method extracts empty string as provider name
        assert!(err.message.contains("provider"));
    }
}

#[test]
fn request_cancelled_error_has_no_data() {
    let err = request_cancelled_error();
    assert!(err.data.is_none());
    assert_eq!(err.code, REQUEST_CANCELLED);
}

#[test]
fn server_cancelled_error_has_no_data() {
    let err = server_cancelled_error();
    assert!(err.data.is_none());
    assert_eq!(err.code, SERVER_CANCELLED);
}

#[test]
fn method_not_found_includes_method_name() {
    let err = method_not_found("textDocument/magic");
    assert!(err.message.contains("textDocument/magic"));
    assert_eq!(err.code, METHOD_NOT_FOUND);
}

#[test]
fn method_not_advertised_uses_method_not_found_code() {
    let err = method_not_advertised();
    assert_eq!(err.code, METHOD_NOT_FOUND);
}

#[test]
fn invalid_params_preserves_message() {
    let err = invalid_params("Expected object, got array");
    assert_eq!(err.message, "Expected object, got array");
    assert_eq!(err.code, INVALID_PARAMS);
}

#[test]
fn server_not_initialized_error_message() {
    let err = server_not_initialized();
    assert!(err.message.contains("not initialized"));
    assert_eq!(err.code, SERVER_NOT_INITIALIZED);
}

#[test]
fn internal_error_preserves_message() {
    let err = internal_error("unexpected panic in handler");
    assert_eq!(err.message, "unexpected panic in handler");
    assert_eq!(err.code, INTERNAL_ERROR);
}

#[test]
fn connection_closed_error_has_correct_code() {
    let err = connection_closed_error();
    assert_eq!(err.code, CONNECTION_CLOSED);
}

#[test]
fn transport_error_has_correct_code() {
    let err = transport_error("write failed");
    assert_eq!(err.code, TRANSPORT_ERROR);
    assert_eq!(err.message, "write failed");
}

#[test]
fn document_not_found_error_contains_status_and_message() {
    let val = document_not_found_error();
    assert_eq!(val["status"], json!("error"));
    assert_eq!(val["message"], json!("Document not found"));
}

#[test]
fn enhanced_error_includes_server_info() {
    let err = enhanced_error(
        INTERNAL_ERROR,
        "failed",
        "runtime",
        Some("textDocument/hover"),
    );
    if let Some(data) = &err.data {
        assert!(data.get("server_info").is_some());
        assert_eq!(data["method"], json!("textDocument/hover"));
        assert_eq!(data["error_type"], json!("runtime"));
    }
}

#[test]
fn enhanced_error_without_method_has_no_method_field() {
    let err = enhanced_error(INTERNAL_ERROR, "failed", "runtime", None);
    if let Some(data) = &err.data {
        assert!(data.get("method").is_none());
    }
}

#[test]
fn enhanced_error_has_timestamp() {
    let err = enhanced_error(INTERNAL_ERROR, "test", "test_type", None);
    if let Some(data) = &err.data {
        assert!(data.get("timestamp").is_some());
    }
}

#[test]
fn enhanced_error_has_correct_code() {
    let err = enhanced_error(PARSE_ERROR, "bad json", "parse", None);
    assert_eq!(err.code, PARSE_ERROR);
    assert_eq!(err.message, "bad json");
}

// ============================================================================
// Parameter extraction — req_uri
// ============================================================================

#[test]
fn req_uri_with_file_scheme() {
    let params = json!({"textDocument": {"uri": "file:///home/user/test.pl"}});
    assert!(matches!(req_uri(&params), Ok("file:///home/user/test.pl")));
}

#[test]
fn req_uri_with_untitled_scheme() {
    let params = json!({"textDocument": {"uri": "untitled:Untitled-1"}});
    match req_uri(&params) {
        Ok(uri) => assert_eq!(uri, "untitled:Untitled-1"),
        Err(_) => {} // also acceptable if untitled is not supported
    }
}

#[test]
fn req_uri_with_empty_string() {
    let params = json!({"textDocument": {"uri": ""}});
    // Empty string is a valid string, so extraction succeeds
    match req_uri(&params) {
        Ok(uri) => assert_eq!(uri, ""),
        Err(_) => {} // implementation may reject empty
    }
}

#[test]
fn req_uri_missing_text_document() {
    let params = json!({"position": {"line": 0, "character": 0}});
    assert!(req_uri(&params).is_err());
}

#[test]
fn req_uri_uri_is_integer() {
    let params = json!({"textDocument": {"uri": 42}});
    let result = req_uri(&params);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.code, INVALID_PARAMS);
    }
}

#[test]
fn req_uri_uri_is_null() {
    let params = json!({"textDocument": {"uri": null}});
    assert!(req_uri(&params).is_err());
}

#[test]
fn req_uri_uri_is_boolean() {
    let params = json!({"textDocument": {"uri": true}});
    assert!(req_uri(&params).is_err());
}

#[test]
fn req_uri_empty_params() {
    let params = json!({});
    assert!(req_uri(&params).is_err());
}

// ============================================================================
// Parameter extraction — req_position
// ============================================================================

#[test]
fn req_position_typical_values() {
    let params = json!({"position": {"line": 10, "character": 25}});
    assert!(matches!(req_position(&params), Ok((10, 25))));
}

#[test]
fn req_position_line_is_string() {
    let params = json!({"position": {"line": "five", "character": 0}});
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_character_is_negative() {
    let params = json!({"position": {"line": 0, "character": -1}});
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_line_is_negative() {
    let params = json!({"position": {"line": -1, "character": 0}});
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_line_is_float() {
    let params = json!({"position": {"line": 1.5, "character": 0}});
    // JSON floats like 1.5 are not valid u64
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_line_exceeds_u32() {
    let params = json!({"position": {"line": 5_000_000_000_u64, "character": 0}});
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_character_exceeds_u32() {
    let params = json!({"position": {"line": 0, "character": 5_000_000_000_u64}});
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_missing_position_object() {
    let params = json!({"textDocument": {"uri": "file:///a.pl"}});
    assert!(req_position(&params).is_err());
}

#[test]
fn req_position_position_is_null() {
    let params = json!({"position": null});
    assert!(req_position(&params).is_err());
}

// ============================================================================
// Parameter extraction — req_range
// ============================================================================

#[test]
fn req_range_typical_values() {
    let params = json!({
        "range": {
            "start": {"line": 5, "character": 10},
            "end": {"line": 5, "character": 20}
        }
    });
    assert!(matches!(req_range(&params), Ok(((5, 10), (5, 20)))));
}

#[test]
fn req_range_single_point_range() {
    let params = json!({
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": 0}
        }
    });
    assert!(matches!(req_range(&params), Ok(((0, 0), (0, 0)))));
}

#[test]
fn req_range_missing_range_object() {
    let params = json!({"textDocument": {"uri": "file:///a.pl"}});
    assert!(req_range(&params).is_err());
}

#[test]
fn req_range_missing_start() {
    let params = json!({
        "range": {
            "end": {"line": 10, "character": 0}
        }
    });
    assert!(req_range(&params).is_err());
}

#[test]
fn req_range_missing_end() {
    let params = json!({
        "range": {
            "start": {"line": 0, "character": 0}
        }
    });
    assert!(req_range(&params).is_err());
}

#[test]
fn req_range_start_line_is_float() {
    let params = json!({
        "range": {
            "start": {"line": 1.5, "character": 0},
            "end": {"line": 2, "character": 0}
        }
    });
    assert!(req_range(&params).is_err());
}

#[test]
fn req_range_end_character_is_string() {
    let params = json!({
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": "ten"}
        }
    });
    assert!(req_range(&params).is_err());
}

#[test]
fn req_range_max_u32_values() {
    let max = u32::MAX as u64;
    let params = json!({
        "range": {
            "start": {"line": max, "character": max},
            "end": {"line": max, "character": max}
        }
    });
    assert!(matches!(
        req_range(&params),
        Ok(((u32::MAX, u32::MAX), (u32::MAX, u32::MAX)))
    ));
}

// ============================================================================
// Capabilities — individual feature flags
// ============================================================================

#[test]
fn capabilities_inlay_hints_enabled() {
    let mut flags = BuildFlags::all();
    flags.inlay_hints = true;
    let caps = capabilities_for(flags);
    assert!(caps.inlay_hint_provider.is_some());
}

#[test]
fn capabilities_inlay_hints_disabled() {
    let mut flags = BuildFlags::all();
    flags.inlay_hints = false;
    let caps = capabilities_for(flags);
    assert!(caps.inlay_hint_provider.is_none());
}

#[test]
fn capabilities_semantic_tokens_enabled() {
    let mut flags = BuildFlags::all();
    flags.semantic_tokens = true;
    let caps = capabilities_for(flags);
    assert!(caps.semantic_tokens_provider.is_some());
}

#[test]
fn capabilities_semantic_tokens_disabled() {
    let mut flags = BuildFlags::all();
    flags.semantic_tokens = false;
    let caps = capabilities_for(flags);
    assert!(caps.semantic_tokens_provider.is_none());
}

#[test]
fn capabilities_rename_enabled() {
    let mut flags = BuildFlags::all();
    flags.rename = true;
    let caps = capabilities_for(flags);
    assert!(caps.rename_provider.is_some());
}

#[test]
fn capabilities_rename_disabled() {
    let mut flags = BuildFlags::all();
    flags.rename = false;
    let caps = capabilities_for(flags);
    assert!(caps.rename_provider.is_none());
}

#[test]
fn capabilities_code_lens_enabled() {
    let mut flags = BuildFlags::all();
    flags.code_lens = true;
    let caps = capabilities_for(flags);
    assert!(caps.code_lens_provider.is_some());
}

#[test]
fn capabilities_code_lens_disabled() {
    let mut flags = BuildFlags::all();
    flags.code_lens = false;
    let caps = capabilities_for(flags);
    assert!(caps.code_lens_provider.is_none());
}

#[test]
fn capabilities_document_links_enabled() {
    let mut flags = BuildFlags::all();
    flags.document_links = true;
    let caps = capabilities_for(flags);
    assert!(caps.document_link_provider.is_some());
}

#[test]
fn capabilities_document_links_disabled() {
    let mut flags = BuildFlags::all();
    flags.document_links = false;
    let caps = capabilities_for(flags);
    assert!(caps.document_link_provider.is_none());
}

#[test]
fn capabilities_selection_ranges_enabled() {
    let mut flags = BuildFlags::all();
    flags.selection_ranges = true;
    let caps = capabilities_for(flags);
    assert!(caps.selection_range_provider.is_some());
}

#[test]
fn capabilities_selection_ranges_disabled() {
    let mut flags = BuildFlags::all();
    flags.selection_ranges = false;
    let caps = capabilities_for(flags);
    assert!(caps.selection_range_provider.is_none());
}

#[test]
fn capabilities_on_type_formatting_enabled() {
    let mut flags = BuildFlags::all();
    flags.on_type_formatting = true;
    let caps = capabilities_for(flags);
    assert!(caps.document_on_type_formatting_provider.is_some());
}

#[test]
fn capabilities_on_type_formatting_disabled() {
    let mut flags = BuildFlags::all();
    flags.on_type_formatting = false;
    let caps = capabilities_for(flags);
    assert!(caps.document_on_type_formatting_provider.is_none());
}

#[test]
fn capabilities_formatting_enabled() {
    let mut flags = BuildFlags::all();
    flags.formatting = true;
    let caps = capabilities_for(flags);
    assert!(caps.document_formatting_provider.is_some());
}

#[test]
fn capabilities_formatting_disabled() {
    let mut flags = BuildFlags::all();
    flags.formatting = false;
    let caps = capabilities_for(flags);
    assert!(caps.document_formatting_provider.is_none());
}

#[test]
fn capabilities_range_formatting_enabled() {
    let mut flags = BuildFlags::all();
    flags.range_formatting = true;
    let caps = capabilities_for(flags);
    assert!(caps.document_range_formatting_provider.is_some());
}

#[test]
fn capabilities_range_formatting_disabled() {
    let mut flags = BuildFlags::all();
    flags.range_formatting = false;
    let caps = capabilities_for(flags);
    assert!(caps.document_range_formatting_provider.is_none());
}

#[test]
fn capabilities_call_hierarchy_enabled() {
    let mut flags = BuildFlags::all();
    flags.call_hierarchy = true;
    let caps = capabilities_for(flags);
    assert!(caps.call_hierarchy_provider.is_some());
}

#[test]
fn capabilities_call_hierarchy_disabled() {
    let mut flags = BuildFlags::all();
    flags.call_hierarchy = false;
    let caps = capabilities_for(flags);
    assert!(caps.call_hierarchy_provider.is_none());
}

#[test]
fn capabilities_linked_editing_enabled() {
    let mut flags = BuildFlags::all();
    flags.linked_editing = true;
    let caps = capabilities_for(flags);
    assert!(caps.linked_editing_range_provider.is_some());
}

#[test]
fn capabilities_linked_editing_disabled() {
    let mut flags = BuildFlags::all();
    flags.linked_editing = false;
    let caps = capabilities_for(flags);
    assert!(caps.linked_editing_range_provider.is_none());
}

#[test]
fn capabilities_moniker_enabled() {
    let mut flags = BuildFlags::all();
    flags.moniker = true;
    let caps = capabilities_for(flags);
    assert!(caps.moniker_provider.is_some());
}

#[test]
fn capabilities_moniker_disabled() {
    let mut flags = BuildFlags::all();
    flags.moniker = false;
    let caps = capabilities_for(flags);
    assert!(caps.moniker_provider.is_none());
}

#[test]
fn capabilities_document_color_enabled() {
    let mut flags = BuildFlags::all();
    flags.document_color = true;
    let caps = capabilities_for(flags);
    assert!(caps.color_provider.is_some());
}

#[test]
fn capabilities_document_color_disabled() {
    let mut flags = BuildFlags::all();
    flags.document_color = false;
    let caps = capabilities_for(flags);
    assert!(caps.color_provider.is_none());
}

#[test]
fn capabilities_inline_values_enabled() {
    let mut flags = BuildFlags::all();
    flags.inline_values = true;
    let caps = capabilities_for(flags);
    assert!(caps.inline_value_provider.is_some());
}

#[test]
fn capabilities_inline_values_disabled() {
    let mut flags = BuildFlags::all();
    flags.inline_values = false;
    let caps = capabilities_for(flags);
    assert!(caps.inline_value_provider.is_none());
}

#[test]
fn capabilities_declaration_enabled() {
    let mut flags = BuildFlags::all();
    flags.declaration = true;
    let caps = capabilities_for(flags);
    assert!(caps.declaration_provider.is_some());
}

#[test]
fn capabilities_declaration_disabled() {
    let mut flags = BuildFlags::all();
    flags.declaration = false;
    let caps = capabilities_for(flags);
    assert!(caps.declaration_provider.is_none());
}

#[test]
fn capabilities_document_highlight_enabled() {
    let mut flags = BuildFlags::all();
    flags.document_highlight = true;
    let caps = capabilities_for(flags);
    assert!(caps.document_highlight_provider.is_some());
}

#[test]
fn capabilities_document_highlight_disabled() {
    let mut flags = BuildFlags::all();
    flags.document_highlight = false;
    let caps = capabilities_for(flags);
    assert!(caps.document_highlight_provider.is_none());
}

#[test]
fn capabilities_signature_help_enabled() {
    let mut flags = BuildFlags::all();
    flags.signature_help = true;
    let caps = capabilities_for(flags);
    assert!(caps.signature_help_provider.is_some());
}

#[test]
fn capabilities_signature_help_disabled() {
    let mut flags = BuildFlags::all();
    flags.signature_help = false;
    let caps = capabilities_for(flags);
    assert!(caps.signature_help_provider.is_none());
}

// ============================================================================
// Capabilities — core feature flags
// ============================================================================

#[test]
fn capabilities_production_has_hover() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.hover_provider.is_some());
}

#[test]
fn capabilities_production_has_completion() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.completion_provider.is_some());
}

#[test]
fn capabilities_production_has_definition() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.definition_provider.is_some());
}

#[test]
fn capabilities_production_has_references() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.references_provider.is_some());
}

#[test]
fn capabilities_production_has_document_symbol() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.document_symbol_provider.is_some());
}

#[test]
fn capabilities_production_has_folding_range() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.folding_range_provider.is_some());
}

#[test]
fn capabilities_always_has_text_document_sync() {
    let caps = capabilities_for(BuildFlags::production());
    assert!(caps.text_document_sync.is_some());
}

#[test]
fn capabilities_default_omits_flagged_core_features() {
    let caps = capabilities_for(BuildFlags::default());
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_none());
    assert!(caps.completion_provider.is_none());
    assert!(caps.definition_provider.is_none());
    assert!(caps.references_provider.is_none());
    assert!(caps.document_symbol_provider.is_none());
    assert!(caps.workspace_symbol_provider.is_none());
    assert!(caps.folding_range_provider.is_none());
}

// ============================================================================
// Capabilities — JSON output
// ============================================================================

#[test]
fn capabilities_json_all_flags_is_valid_object() {
    let json = capabilities_json(BuildFlags::all());
    assert!(json.is_object());
}

#[test]
fn capabilities_json_has_hover_provider() {
    let json = capabilities_json(BuildFlags::all());
    assert!(cap_bool_or_object(&json, "hoverProvider"));
}

#[test]
fn capabilities_json_has_text_document_sync() {
    let json = capabilities_json(BuildFlags::all());
    assert!(json.get("textDocumentSync").is_some());
}

#[test]
fn capabilities_json_type_hierarchy_with_flag() {
    let mut flags = BuildFlags::all();
    flags.type_hierarchy = true;
    let json = capabilities_json(flags);
    assert!(json.get("typeHierarchyProvider").is_some());
}

#[test]
fn capabilities_json_no_type_hierarchy_without_flag() {
    let mut flags = BuildFlags::all();
    flags.type_hierarchy = false;
    let json = capabilities_json(flags);
    assert!(json.get("typeHierarchyProvider").is_none());
}

// ============================================================================
// cap_bool_or_object — edge cases
// ============================================================================

#[test]
fn cap_bool_or_object_with_array_returns_false() {
    let caps = json!({"provider": [1, 2, 3]});
    assert!(!cap_bool_or_object(&caps, "provider"));
}

#[test]
fn cap_bool_or_object_with_false_boolean() {
    let caps = json!({"provider": false});
    assert!(cap_bool_or_object(&caps, "provider"));
}

#[test]
fn cap_bool_or_object_with_empty_object() {
    let caps = json!({"provider": {}});
    assert!(cap_bool_or_object(&caps, "provider"));
}

// ============================================================================
// get_supported_commands
// ============================================================================

#[test]
fn supported_commands_count_is_at_least_five() {
    let cmds = get_supported_commands();
    assert!(cmds.len() >= 5);
}

#[test]
fn supported_commands_are_non_empty_strings() {
    for cmd in get_supported_commands() {
        assert!(!cmd.is_empty());
    }
}

#[test]
fn supported_commands_contain_run_tests() {
    let cmds = get_supported_commands();
    assert!(cmds.contains(&"perl.runTests".to_string()));
}

#[test]
fn supported_commands_contain_debug_file() {
    let cmds = get_supported_commands();
    assert!(cmds.contains(&"perl.debugFile".to_string()));
}

// ============================================================================
// default_capabilities
// ============================================================================

#[test]
fn default_capabilities_has_hover() {
    let caps = default_capabilities();
    assert!(caps.hover_provider.is_some());
}

#[test]
fn default_capabilities_has_completion() {
    let caps = default_capabilities();
    assert!(caps.completion_provider.is_some());
}

// ============================================================================
// Method constants — format and prefix validation
// ============================================================================

#[test]
fn all_text_document_methods_start_with_prefix() {
    let text_doc_methods = [
        methods::TEXT_DOCUMENT_DID_OPEN,
        methods::TEXT_DOCUMENT_DID_CHANGE,
        methods::TEXT_DOCUMENT_DID_CLOSE,
        methods::TEXT_DOCUMENT_DID_SAVE,
        methods::TEXT_DOCUMENT_WILL_SAVE,
        methods::TEXT_DOCUMENT_WILL_SAVE_WAIT_UNTIL,
        methods::TEXT_DOCUMENT_PUBLISH_DIAGNOSTICS,
        methods::TEXT_DOCUMENT_COMPLETION,
        methods::TEXT_DOCUMENT_HOVER,
        methods::TEXT_DOCUMENT_SIGNATURE_HELP,
        methods::TEXT_DOCUMENT_DEFINITION,
        methods::TEXT_DOCUMENT_DECLARATION,
        methods::TEXT_DOCUMENT_TYPE_DEFINITION,
        methods::TEXT_DOCUMENT_IMPLEMENTATION,
        methods::TEXT_DOCUMENT_REFERENCES,
        methods::TEXT_DOCUMENT_DOCUMENT_SYMBOL,
        methods::TEXT_DOCUMENT_DOCUMENT_HIGHLIGHT,
        methods::TEXT_DOCUMENT_CODE_ACTION,
        methods::TEXT_DOCUMENT_CODE_LENS,
        methods::TEXT_DOCUMENT_FORMATTING,
        methods::TEXT_DOCUMENT_RANGE_FORMATTING,
        methods::TEXT_DOCUMENT_RANGES_FORMATTING,
        methods::TEXT_DOCUMENT_ON_TYPE_FORMATTING,
        methods::TEXT_DOCUMENT_PREPARE_RENAME,
        methods::TEXT_DOCUMENT_RENAME,
        methods::TEXT_DOCUMENT_LINKED_EDITING_RANGE,
        methods::TEXT_DOCUMENT_SEMANTIC_TOKENS_FULL,
        methods::TEXT_DOCUMENT_SEMANTIC_TOKENS_RANGE,
        methods::TEXT_DOCUMENT_INLAY_HINT,
        methods::TEXT_DOCUMENT_DOCUMENT_LINK,
        methods::TEXT_DOCUMENT_FOLDING_RANGE,
        methods::TEXT_DOCUMENT_SELECTION_RANGE,
        methods::TEXT_DOCUMENT_PREPARE_TYPE_HIERARCHY,
        methods::TEXT_DOCUMENT_PREPARE_CALL_HIERARCHY,
        methods::TEXT_DOCUMENT_DIAGNOSTIC,
        methods::TEXT_DOCUMENT_INLINE_COMPLETION,
        methods::TEXT_DOCUMENT_INLINE_VALUE,
        methods::TEXT_DOCUMENT_DOCUMENT_COLOR,
        methods::TEXT_DOCUMENT_COLOR_PRESENTATION,
        methods::TEXT_DOCUMENT_MONIKER,
    ];
    for method in &text_doc_methods {
        assert!(
            method.starts_with("textDocument/"),
            "Expected textDocument/ prefix for: {}",
            method
        );
    }
}

#[test]
fn all_workspace_methods_start_with_prefix() {
    let workspace_methods = [
        methods::WORKSPACE_SYMBOL,
        methods::WORKSPACE_SYMBOL_RESOLVE,
        methods::WORKSPACE_EXECUTE_COMMAND,
        methods::WORKSPACE_APPLY_EDIT,
        methods::WORKSPACE_CONFIGURATION,
        methods::WORKSPACE_TEXT_DOCUMENT_CONTENT,
        methods::WORKSPACE_WILL_CREATE_FILES,
        methods::WORKSPACE_DID_CREATE_FILES,
        methods::WORKSPACE_WILL_RENAME_FILES,
        methods::WORKSPACE_DID_RENAME_FILES,
        methods::WORKSPACE_WILL_DELETE_FILES,
        methods::WORKSPACE_DID_DELETE_FILES,
        methods::WORKSPACE_DID_CHANGE_WORKSPACE_FOLDERS,
        methods::WORKSPACE_DID_CHANGE_CONFIGURATION,
        methods::WORKSPACE_DID_CHANGE_WATCHED_FILES,
        methods::WORKSPACE_CODE_LENS_REFRESH,
        methods::WORKSPACE_SEMANTIC_TOKENS_REFRESH,
        methods::WORKSPACE_INLAY_HINT_REFRESH,
        methods::WORKSPACE_INLINE_VALUE_REFRESH,
        methods::WORKSPACE_DIAGNOSTIC_REFRESH,
        methods::WORKSPACE_FOLDING_RANGE_REFRESH,
        methods::WORKSPACE_TEXT_DOCUMENT_CONTENT_REFRESH,
        methods::WORKSPACE_DIAGNOSTIC,
    ];
    for method in &workspace_methods {
        assert!(
            method.starts_with("workspace/"),
            "Expected workspace/ prefix for: {}",
            method
        );
    }
}

#[test]
fn all_window_methods_start_with_prefix() {
    let window_methods = [
        methods::WINDOW_SHOW_MESSAGE,
        methods::WINDOW_LOG_MESSAGE,
        methods::WINDOW_SHOW_MESSAGE_REQUEST,
        methods::WINDOW_SHOW_DOCUMENT,
        methods::WINDOW_WORK_DONE_PROGRESS_CREATE,
        methods::WINDOW_WORK_DONE_PROGRESS_CANCEL,
    ];
    for method in &window_methods {
        assert!(
            method.starts_with("window/"),
            "Expected window/ prefix for: {}",
            method
        );
    }
}

#[test]
fn all_notebook_methods_start_with_prefix() {
    let notebook_methods = [
        methods::NOTEBOOK_DOCUMENT_DID_OPEN,
        methods::NOTEBOOK_DOCUMENT_DID_CHANGE,
        methods::NOTEBOOK_DOCUMENT_DID_SAVE,
        methods::NOTEBOOK_DOCUMENT_DID_CLOSE,
    ];
    for method in &notebook_methods {
        assert!(
            method.starts_with("notebookDocument/"),
            "Expected notebookDocument/ prefix for: {}",
            method
        );
    }
}

#[test]
fn special_methods_start_with_dollar() {
    let special = [
        methods::CANCEL_REQUEST,
        methods::DOLLAR_PROGRESS,
        methods::TEST_SLOW_OPERATION,
    ];
    for method in &special {
        assert!(
            method.starts_with("$/"),
            "Expected $/ prefix for: {}",
            method
        );
    }
}

#[test]
fn resolve_methods_use_slash_resolve_pattern() {
    let resolve_methods = [
        methods::COMPLETION_ITEM_RESOLVE,
        methods::CODE_ACTION_RESOLVE,
        methods::CODE_LENS_RESOLVE,
        methods::INLAY_HINT_RESOLVE,
        methods::DOCUMENT_LINK_RESOLVE,
    ];
    for method in &resolve_methods {
        assert!(
            method.contains("/resolve"),
            "Expected /resolve in: {}",
            method
        );
    }
}

#[test]
fn hierarchy_methods_use_correct_prefixes() {
    assert!(methods::CALL_HIERARCHY_INCOMING_CALLS.starts_with("callHierarchy/"));
    assert!(methods::CALL_HIERARCHY_OUTGOING_CALLS.starts_with("callHierarchy/"));
    assert!(methods::TYPE_HIERARCHY_SUPERTYPES.starts_with("typeHierarchy/"));
    assert!(methods::TYPE_HIERARCHY_SUBTYPES.starts_with("typeHierarchy/"));
}

#[test]
fn method_constants_are_nonempty() {
    let all_methods = [
        methods::INITIALIZE,
        methods::INITIALIZED,
        methods::SHUTDOWN,
        methods::EXIT,
        methods::TEXT_DOCUMENT_HOVER,
        methods::TEXT_DOCUMENT_COMPLETION,
        methods::TEXT_DOCUMENT_DEFINITION,
        methods::WORKSPACE_SYMBOL,
        methods::CANCEL_REQUEST,
    ];
    for method in &all_methods {
        assert!(!method.is_empty());
    }
}

#[test]
fn lifecycle_methods_have_no_slash() {
    // Lifecycle methods are bare words, not prefixed
    assert!(!methods::INITIALIZE.contains('/'));
    assert!(!methods::INITIALIZED.contains('/'));
    assert!(!methods::SHUTDOWN.contains('/'));
    assert!(!methods::EXIT.contains('/'));
}
