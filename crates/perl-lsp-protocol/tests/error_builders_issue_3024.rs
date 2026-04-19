//! Tests for LSP error response builders — closes #3024
//!
//! Covers serialisation round-trips, JSON-RPC conformance, and edge cases
//! for all public functions in `perl-lsp-protocol/src/errors.rs` that were
//! not yet individually exercised elsewhere.

use perl_lsp_protocol::*;
use serde_json::{Value, json};

// ============================================================================
// Full JSON-RPC serialization round-trips
// ============================================================================

#[test]
fn cancelled_response_roundtrip_produces_valid_jsonrpc() -> Result<(), Box<dyn std::error::Error>> {
    let id = json!(42);
    let resp = cancelled_response(&id);
    let serialized = serde_json::to_value(&resp)?;

    assert_eq!(serialized["jsonrpc"], "2.0");
    assert_eq!(serialized["id"], 42);
    assert_eq!(serialized["error"]["code"], REQUEST_CANCELLED);
    assert_eq!(serialized["error"]["message"], "Request cancelled");
    // "result" must be absent per JSON-RPC spec
    assert!(
        !serialized
            .as_object()
            .is_some_and(|o| o.contains_key("result"))
    );
    Ok(())
}

#[test]
fn cancelled_response_with_method_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let id = json!("req-007");
    let resp = cancelled_response_with_method(&id, "textDocument/completion");
    let serialized = serde_json::to_value(&resp)?;

    assert_eq!(serialized["jsonrpc"], "2.0");
    assert_eq!(serialized["id"], "req-007");
    assert_eq!(serialized["error"]["code"], REQUEST_CANCELLED);
    let msg = serialized["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("completion"),
        "message must contain provider name"
    );
    let data = &serialized["error"]["data"];
    assert_eq!(data["provider"], "textDocument/completion");
    assert!(data["timestamp"].is_number(), "timestamp must be numeric");
    Ok(())
}

// ============================================================================
// cancelled_response_with_method — provider name extraction logic
// ============================================================================

#[test]
fn cancelled_response_with_method_three_segment_path() {
    let resp = cancelled_response_with_method(&json!(1), "textDocument/semanticTokens/full/delta");
    assert!(
        resp.error.is_some(),
        "cancelled_response_with_method must include an error"
    );
    if let Some(err) = resp.error.as_ref() {
        // The last segment "delta" is extracted as the provider name
        assert!(
            err.message.contains("delta"),
            "provider name 'delta' must appear in message"
        );
    }
}

#[test]
fn cancelled_response_with_method_result_is_always_none() {
    let resp = cancelled_response_with_method(&json!(5), "workspace/symbol");
    assert!(
        resp.result.is_none(),
        "result must be None in cancelled response"
    );
}

#[test]
fn cancelled_response_with_method_data_has_request_id() {
    let id = json!(99);
    let resp = cancelled_response_with_method(&id, "textDocument/hover");
    assert!(
        resp.error.as_ref().and_then(|e| e.data.as_ref()).is_some(),
        "data must be present in cancelled_response_with_method"
    );
    if let Some(data) = resp.error.as_ref().and_then(|e| e.data.as_ref()) {
        assert_eq!(
            data["request_id"], id,
            "request_id in data must match the original id"
        );
    }
}

// ============================================================================
// req_uri — INVALID_PARAMS error code in all error cases
// ============================================================================

#[test]
fn req_uri_missing_uri_key_returns_invalid_params() {
    let params = json!({ "textDocument": {} });
    let result = req_uri(&params);
    assert!(
        result.is_err(),
        "req_uri must return Err for missing URI key"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
    }
}

#[test]
fn req_uri_array_value_returns_invalid_params() {
    let params = json!({ "textDocument": { "uri": ["not", "a", "string"] } });
    let result = req_uri(&params);
    assert!(
        result.is_err(),
        "req_uri must return Err for array URI value"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
    }
}

#[test]
fn req_uri_object_value_returns_invalid_params() {
    let params = json!({ "textDocument": { "uri": { "nested": true } } });
    let result = req_uri(&params);
    assert!(
        result.is_err(),
        "req_uri must return Err for object URI value"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
    }
}

#[test]
fn req_uri_empty_string_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    // Empty string is a valid string value — req_uri returns it unchanged
    let params = json!({ "textDocument": { "uri": "" } });
    let uri = req_uri(&params)?;
    assert_eq!(uri, "");
    Ok(())
}

// ============================================================================
// req_position — error message content
// ============================================================================

#[test]
fn req_position_missing_line_error_message_names_field() {
    let params = json!({ "position": { "character": 5 } });
    let result = req_position(&params);
    assert!(
        result.is_err(),
        "req_position must return Err when line is missing"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("position.line"),
            "error message must name the missing field; got: {}",
            err.message
        );
    }
}

#[test]
fn req_position_missing_character_error_message_names_field() {
    let params = json!({ "position": { "line": 0 } });
    let result = req_position(&params);
    assert!(
        result.is_err(),
        "req_position must return Err when character is missing"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("position.character"),
            "error message must name the missing field; got: {}",
            err.message
        );
    }
}

#[test]
fn req_position_line_overflow_error_message_names_field() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({ "position": { "line": over, "character": 0 } });
    let result = req_position(&params);
    assert!(
        result.is_err(),
        "req_position must return Err when line overflows u32"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("u32"),
            "overflow error must mention u32; got: {}",
            err.message
        );
    }
}

#[test]
fn req_position_character_overflow_error_message_names_field() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({ "position": { "line": 0, "character": over } });
    let result = req_position(&params);
    assert!(
        result.is_err(),
        "req_position must return Err when character overflows u32"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("u32"),
            "overflow error must mention u32; got: {}",
            err.message
        );
    }
}

// ============================================================================
// req_range — missing individual components produce specific messages
// ============================================================================

#[test]
fn req_range_missing_start_character_names_field() {
    let params = json!({
        "range": {
            "start": { "line": 0 },
            "end": { "line": 1, "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(
        result.is_err(),
        "req_range must return Err when start.character is missing"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("range.start.character"),
            "error must name range.start.character; got: {}",
            err.message
        );
    }
}

#[test]
fn req_range_missing_end_line_names_field() {
    let params = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(
        result.is_err(),
        "req_range must return Err when end.line is missing"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("range.end.line"),
            "error must name range.end.line; got: {}",
            err.message
        );
    }
}

#[test]
fn req_range_overflow_start_character_error_message() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({
        "range": {
            "start": { "line": 0, "character": over },
            "end": { "line": 0, "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(
        result.is_err(),
        "req_range must return Err when start.character overflows u32"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("u32"),
            "overflow error must mention u32; got: {}",
            err.message
        );
    }
}

#[test]
fn req_range_overflow_end_character_error_message() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": over }
        }
    });
    let result = req_range(&params);
    assert!(
        result.is_err(),
        "req_range must return Err when end.character overflows u32"
    );
    if let Err(err) = result {
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("u32"),
            "overflow error must mention u32; got: {}",
            err.message
        );
    }
}

#[test]
fn req_range_zero_range_is_valid() -> Result<(), Box<dyn std::error::Error>> {
    // A zero-length range (start == end) is valid per LSP spec
    let params = json!({
        "range": {
            "start": { "line": 5, "character": 3 },
            "end": { "line": 5, "character": 3 }
        }
    });
    let ((sl, sc), (el, ec)) = req_range(&params)?;
    assert_eq!((sl, sc), (5, 3));
    assert_eq!((el, ec), (5, 3));
    Ok(())
}

// ============================================================================
// Error builder — no-data contracts
// ============================================================================

#[test]
fn method_not_found_has_no_data() {
    let err = method_not_found("some/method");
    assert!(
        err.data.is_none(),
        "method_not_found must not include data field"
    );
}

#[test]
fn method_not_advertised_has_no_data() {
    let err = method_not_advertised();
    assert!(
        err.data.is_none(),
        "method_not_advertised must not include data field"
    );
}

#[test]
fn server_not_initialized_has_no_data() {
    let err = server_not_initialized();
    assert!(
        err.data.is_none(),
        "server_not_initialized must not include data field"
    );
}

#[test]
fn invalid_params_has_no_data() {
    let err = invalid_params("test");
    assert!(
        err.data.is_none(),
        "invalid_params must not include data field"
    );
}

#[test]
fn internal_error_has_no_data() {
    let err = internal_error("test");
    assert!(
        err.data.is_none(),
        "internal_error must not include data field"
    );
}

#[test]
fn connection_closed_error_has_no_data() {
    let err = connection_closed_error();
    assert!(
        err.data.is_none(),
        "connection_closed_error must not include data field"
    );
}

#[test]
fn transport_error_has_no_data() {
    let err = transport_error("some I/O failure");
    assert!(
        err.data.is_none(),
        "transport_error must not include data field"
    );
}

// ============================================================================
// Error builder — JSON serialisation produces correct field names
// ============================================================================

#[test]
fn error_json_fields_use_correct_names() -> Result<(), Box<dyn std::error::Error>> {
    let err = internal_error("boom");
    let v: Value = serde_json::to_value(&err)?;
    assert!(v.as_object().is_some_and(|o| o.contains_key("code")));
    assert!(v.as_object().is_some_and(|o| o.contains_key("message")));
    Ok(())
}

#[test]
fn transport_error_serialises_code_and_message() -> Result<(), Box<dyn std::error::Error>> {
    let err = transport_error("disk full");
    let v: Value = serde_json::to_value(&err)?;
    assert_eq!(v["code"], TRANSPORT_ERROR);
    assert_eq!(v["message"], "disk full");
    Ok(())
}

// ============================================================================
// document_not_found_error — value structure
// ============================================================================

#[test]
fn document_not_found_error_is_object() {
    let v = document_not_found_error();
    assert!(
        v.is_object(),
        "document_not_found_error must return a JSON object"
    );
}

#[test]
fn document_not_found_error_message_is_string() {
    let v = document_not_found_error();
    assert!(
        v["message"].is_string(),
        "document_not_found_error message field must be a string"
    );
}

// ============================================================================
// enhanced_error — server_info sub-fields
// ============================================================================

#[test]
fn enhanced_error_server_info_has_name() {
    let err = enhanced_error(INTERNAL_ERROR, "test", "TestError", None);
    assert!(err.data.is_some(), "enhanced_error must include data");
    if let Some(data) = err.data.as_ref() {
        let name = data["server_info"]["name"].as_str().unwrap_or_default();
        assert!(!name.is_empty(), "server_info.name must be non-empty");
    }
}

#[test]
fn enhanced_error_server_info_has_version() {
    let err = enhanced_error(INTERNAL_ERROR, "test", "TestError", None);
    assert!(err.data.is_some(), "enhanced_error must include data");
    if let Some(data) = err.data.as_ref() {
        let version = data["server_info"]["version"].as_str().unwrap_or_default();
        assert!(!version.is_empty(), "server_info.version must be non-empty");
    }
}

#[test]
fn enhanced_error_error_type_field_matches_input() {
    let err = enhanced_error(INTERNAL_ERROR, "msg", "CustomErrorKind", None);
    assert!(err.data.is_some(), "enhanced_error must include data");
    if let Some(data) = err.data.as_ref() {
        assert_eq!(data["error_type"], "CustomErrorKind");
    }
}

#[test]
fn enhanced_error_with_method_stores_full_method_path() {
    let err = enhanced_error(
        METHOD_NOT_FOUND,
        "nope",
        "MethodNotFound",
        Some("$/progress"),
    );
    assert!(err.data.is_some(), "enhanced_error must include data");
    if let Some(data) = err.data.as_ref() {
        assert_eq!(data["method"], "$/progress");
    }
}
