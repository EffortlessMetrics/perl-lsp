//! Comprehensive unit tests for perl-lsp-protocol crate.
//!
//! Covers: JSON-RPC message types, error codes, error builders,
//! parameter extraction helpers, capabilities, and method constants.
#![allow(clippy::assertions_on_constants)]

use perl_lsp_protocol::*;
use serde_json::{Value, json};

// ============================================================================
// JsonRpcRequest — deserialization
// ============================================================================

#[test]
fn request_deserialize_with_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "textDocument/hover",
        "params": { "textDocument": { "uri": "file:///a.pl" } }
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert_eq!(req.method, "textDocument/hover");
    assert_eq!(req.id, Some(json!(1)));
    assert!(req.params.is_some());
    Ok(())
}

#[test]
fn request_deserialize_notification_no_id() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert!(req.id.is_none());
    assert_eq!(req.method, "initialized");
    Ok(())
}

#[test]
fn request_deserialize_without_params() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "shutdown"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert!(req.params.is_none());
    Ok(())
}

#[test]
fn request_deserialize_string_id() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": "req-abc",
        "method": "initialize"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert_eq!(req.id, Some(json!("req-abc")));
    Ok(())
}

#[test]
fn request_deserialize_null_id() -> Result<(), Box<dyn std::error::Error>> {
    // serde deserializes JSON `null` into `Option::None`
    let raw = json!({
        "jsonrpc": "2.0",
        "id": null,
        "method": "shutdown"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    assert!(req.id.is_none());
    Ok(())
}

// ============================================================================
// JsonRpcResponse — construction & serialization
// ============================================================================

#[test]
fn response_success_serializes_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::success(Some(json!(1)), json!({"result": true}));
    let v = serde_json::to_value(&resp)?;
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["id"], 1);
    assert_eq!(v["result"]["result"], true);
    assert!(v.get("error").is_none());
    Ok(())
}

#[test]
fn response_error_serializes_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let err = JsonRpcError::new(-32600, "Invalid Request");
    let resp = JsonRpcResponse::error(Some(json!(2)), err);
    let v = serde_json::to_value(&resp)?;
    assert_eq!(v["error"]["code"], -32600);
    assert_eq!(v["error"]["message"], "Invalid Request");
    assert!(v.get("result").is_none());
    Ok(())
}

#[test]
fn response_null_serializes_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::null(Some(json!(3)));
    let v = serde_json::to_value(&resp)?;
    assert!(v["result"].is_null());
    assert!(v.get("error").is_none());
    Ok(())
}

#[test]
fn response_success_with_none_id() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::success(None, json!("ok"));
    let v = serde_json::to_value(&resp)?;
    assert!(v["id"].is_null());
    Ok(())
}

#[test]
fn response_error_omits_result_field() -> Result<(), Box<dyn std::error::Error>> {
    let err = JsonRpcError::new(INTERNAL_ERROR, "boom");
    let resp = JsonRpcResponse::error(Some(json!(9)), err);
    let v = serde_json::to_value(&resp)?;
    // skip_serializing_if means "result" key should not appear at all
    assert!(!v.as_object().is_some_and(|o| o.contains_key("result")));
    Ok(())
}

#[test]
fn response_success_omits_error_field() -> Result<(), Box<dyn std::error::Error>> {
    let resp = JsonRpcResponse::success(Some(json!(10)), json!(42));
    let v = serde_json::to_value(&resp)?;
    assert!(!v.as_object().is_some_and(|o| o.contains_key("error")));
    Ok(())
}

// ============================================================================
// JsonRpcError — construction, Display, Error trait
// ============================================================================

#[test]
fn error_new_sets_fields() {
    let err = JsonRpcError::new(-32601, "Method not found");
    assert_eq!(err.code, -32601);
    assert_eq!(err.message, "Method not found");
    assert!(err.data.is_none());
}

#[test]
fn error_with_data_sets_all_fields() {
    let data = json!({"detail": "extra info"});
    let err = JsonRpcError::with_data(-32602, "Invalid params", data.clone());
    assert_eq!(err.code, -32602);
    assert_eq!(err.data, Some(data));
}

#[test]
fn error_display_format() {
    let err = JsonRpcError::new(-32700, "Parse error");
    let display = format!("{}", err);
    assert_eq!(display, "-32700: Parse error");
}

#[test]
fn error_implements_std_error() {
    let err = JsonRpcError::new(INTERNAL_ERROR, "test");
    let _: &dyn std::error::Error = &err;
}

#[test]
fn error_clone_produces_equal_value() {
    let err = JsonRpcError::with_data(-1, "msg", json!("extra"));
    let cloned = err.clone();
    assert_eq!(err.code, cloned.code);
    assert_eq!(err.message, cloned.message);
    assert_eq!(err.data, cloned.data);
}

#[test]
fn error_debug_is_non_empty() {
    let err = JsonRpcError::new(0, "debug test");
    let dbg = format!("{:?}", err);
    assert!(!dbg.is_empty());
}

#[test]
fn error_serializes_to_json() -> Result<(), Box<dyn std::error::Error>> {
    let err = JsonRpcError::with_data(-32600, "bad", json!(null));
    let v = serde_json::to_value(&err)?;
    assert_eq!(v["code"], -32600);
    assert_eq!(v["message"], "bad");
    assert!(v["data"].is_null());
    Ok(())
}

#[test]
fn error_serializes_without_data() -> Result<(), Box<dyn std::error::Error>> {
    let err = JsonRpcError::new(-32601, "nope");
    let v = serde_json::to_value(&err)?;
    // data is always serialised (no skip_serializing_if on JsonRpcError)
    assert_eq!(v["code"], -32601);
    Ok(())
}

// ============================================================================
// Error Code Constants
// ============================================================================

#[test]
fn standard_jsonrpc_error_codes() {
    assert_eq!(PARSE_ERROR, -32700);
    assert_eq!(INVALID_REQUEST, -32600);
    assert_eq!(METHOD_NOT_FOUND, -32601);
    assert_eq!(INVALID_PARAMS, -32602);
    assert_eq!(INTERNAL_ERROR, -32603);
}

#[test]
fn server_error_range_constants() {
    assert_eq!(SERVER_ERROR_START, -32099);
    assert_eq!(SERVER_ERROR_END, -32000);
    assert!(SERVER_ERROR_START <= SERVER_ERROR_END);
}

#[test]
fn lsp_specific_error_codes() {
    assert_eq!(SERVER_CANCELLED, -32802);
    assert_eq!(CONTENT_MODIFIED, -32801);
    assert_eq!(REQUEST_CANCELLED, -32800);
    assert_eq!(REQUEST_FAILED, -32803);
    assert_eq!(SERVER_NOT_INITIALIZED, -32002);
}

#[test]
fn transport_error_codes_in_server_range() {
    assert!(CONNECTION_CLOSED >= SERVER_ERROR_START);
    assert!(CONNECTION_CLOSED <= SERVER_ERROR_END);
    assert!(TRANSPORT_ERROR >= SERVER_ERROR_START);
    assert!(TRANSPORT_ERROR <= SERVER_ERROR_END);
}

#[test]
fn unknown_error_code_in_server_range() {
    assert!(UNKNOWN_ERROR_CODE >= SERVER_ERROR_START);
    assert!(UNKNOWN_ERROR_CODE <= SERVER_ERROR_END);
}

// ============================================================================
// Error Builder Functions
// ============================================================================

#[test]
fn cancelled_response_builder() {
    let id = json!(5);
    let resp = cancelled_response(&id);
    assert_eq!(resp.jsonrpc, "2.0");
    assert_eq!(resp.id, Some(json!(5)));
    assert!(resp.result.is_none());
    let err = resp.error.as_ref();
    assert!(err.is_some());
    let err = err.unwrap_or_else(|| unreachable!());
    assert_eq!(err.code, REQUEST_CANCELLED);
    assert_eq!(err.message, "Request cancelled");
}

#[test]
fn cancelled_response_with_method_contains_provider() {
    let id = json!(7);
    let resp = cancelled_response_with_method(&id, "textDocument/hover");
    let err = resp.error.as_ref();
    assert!(err.is_some());
    let err = err.unwrap_or_else(|| unreachable!());
    assert_eq!(err.code, REQUEST_CANCELLED);
    assert!(err.message.contains("hover"));
    assert!(err.data.is_some());
    let data = err.data.as_ref().unwrap_or_else(|| unreachable!());
    assert_eq!(data["provider"], "textDocument/hover");
    assert!(data["timestamp"].is_number());
}

#[test]
fn cancelled_response_with_method_single_segment() {
    let id = json!(8);
    let resp = cancelled_response_with_method(&id, "shutdown");
    let err = resp.error.as_ref();
    assert!(err.is_some());
    let err = err.unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("shutdown"));
}

#[test]
fn request_cancelled_error_builder() {
    let err = request_cancelled_error();
    assert_eq!(err.code, REQUEST_CANCELLED);
    assert_eq!(err.message, "Request cancelled");
    assert!(err.data.is_none());
}

#[test]
fn server_cancelled_error_builder() {
    let err = server_cancelled_error();
    assert_eq!(err.code, SERVER_CANCELLED);
    assert!(err.message.contains("cancelled"));
    assert!(err.data.is_none());
}

#[test]
fn method_not_found_builder() {
    let err = method_not_found("textDocument/foo");
    assert_eq!(err.code, METHOD_NOT_FOUND);
    assert!(err.message.contains("textDocument/foo"));
}

#[test]
fn method_not_advertised_builder() {
    let err = method_not_advertised();
    assert_eq!(err.code, METHOD_NOT_FOUND);
    assert!(err.message.contains("not advertised"));
}

#[test]
fn invalid_params_builder() {
    let err = invalid_params("missing field X");
    assert_eq!(err.code, INVALID_PARAMS);
    assert!(err.message.contains("missing field X"));
}

#[test]
fn server_not_initialized_builder() {
    let err = server_not_initialized();
    assert_eq!(err.code, SERVER_NOT_INITIALIZED);
    assert!(err.message.to_lowercase().contains("not initialized"));
}

#[test]
fn internal_error_builder() {
    let err = internal_error("something broke");
    assert_eq!(err.code, INTERNAL_ERROR);
    assert!(err.message.contains("something broke"));
}

#[test]
fn connection_closed_error_builder() {
    let err = connection_closed_error();
    assert_eq!(err.code, CONNECTION_CLOSED);
    assert!(err.message.to_lowercase().contains("connection"));
}

#[test]
fn transport_error_builder() {
    let err = transport_error("write failed");
    assert_eq!(err.code, TRANSPORT_ERROR);
    assert!(err.message.contains("write failed"));
}

#[test]
fn document_not_found_error_is_json_value() {
    let v = document_not_found_error();
    assert_eq!(v["status"], "error");
    assert!(v["message"].as_str().is_some_and(|s| s.contains("not found")));
}

#[test]
fn enhanced_error_contains_metadata() {
    let err = enhanced_error(INTERNAL_ERROR, "oops", "RuntimeError", Some("textDocument/hover"));
    assert_eq!(err.code, INTERNAL_ERROR);
    assert_eq!(err.message, "oops");
    let data = err.data.as_ref();
    assert!(data.is_some());
    let data = data.unwrap_or_else(|| unreachable!());
    assert_eq!(data["error_type"], "RuntimeError");
    assert_eq!(data["method"], "textDocument/hover");
    assert!(data["server_info"]["name"].as_str() == Some("perl-lsp"));
    assert!(data["timestamp"].is_number());
}

#[test]
fn enhanced_error_without_method() {
    let err = enhanced_error(PARSE_ERROR, "bad json", "ParseError", None);
    let data = err.data.as_ref();
    assert!(data.is_some());
    let data = data.unwrap_or_else(|| unreachable!());
    assert!(data.get("method").is_none());
}

// ============================================================================
// Parameter Extraction Helpers
// ============================================================================

#[test]
fn req_uri_extracts_valid_uri() -> Result<(), Box<dyn std::error::Error>> {
    let params = json!({ "textDocument": { "uri": "file:///foo.pl" } });
    let uri = req_uri(&params)?;
    assert_eq!(uri, "file:///foo.pl");
    Ok(())
}

#[test]
fn req_uri_returns_error_when_missing() {
    let params = json!({});
    let result = req_uri(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert_eq!(err.code, INVALID_PARAMS);
    assert!(err.message.contains("textDocument.uri"));
}

#[test]
fn req_uri_returns_error_when_uri_is_not_string() {
    let params = json!({ "textDocument": { "uri": 42 } });
    let result = req_uri(&params);
    assert!(result.is_err());
}

#[test]
fn req_position_extracts_valid_position() -> Result<(), Box<dyn std::error::Error>> {
    let params = json!({ "position": { "line": 10, "character": 5 } });
    let (line, character) = req_position(&params)?;
    assert_eq!(line, 10);
    assert_eq!(character, 5);
    Ok(())
}

#[test]
fn req_position_returns_error_when_line_missing() {
    let params = json!({ "position": { "character": 5 } });
    let result = req_position(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("position.line"));
}

#[test]
fn req_position_returns_error_when_character_missing() {
    let params = json!({ "position": { "line": 0 } });
    let result = req_position(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("position.character"));
}

#[test]
fn req_position_returns_error_on_overflow() {
    let over_u32 = u64::from(u32::MAX) + 1;
    let params = json!({ "position": { "line": over_u32, "character": 0 } });
    let result = req_position(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("u32"));
}

#[test]
fn req_position_returns_error_on_character_overflow() {
    let over_u32 = u64::from(u32::MAX) + 1;
    let params = json!({ "position": { "line": 0, "character": over_u32 } });
    let result = req_position(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("u32"));
}

#[test]
fn req_position_zero_values() -> Result<(), Box<dyn std::error::Error>> {
    let params = json!({ "position": { "line": 0, "character": 0 } });
    let (line, character) = req_position(&params)?;
    assert_eq!(line, 0);
    assert_eq!(character, 0);
    Ok(())
}

#[test]
fn req_position_max_u32() -> Result<(), Box<dyn std::error::Error>> {
    let params = json!({ "position": { "line": u32::MAX, "character": u32::MAX } });
    let (line, character) = req_position(&params)?;
    assert_eq!(line, u32::MAX);
    assert_eq!(character, u32::MAX);
    Ok(())
}

#[test]
fn req_range_extracts_valid_range() -> Result<(), Box<dyn std::error::Error>> {
    let params = json!({
        "range": {
            "start": { "line": 1, "character": 2 },
            "end": { "line": 3, "character": 4 }
        }
    });
    let ((sl, sc), (el, ec)) = req_range(&params)?;
    assert_eq!((sl, sc), (1, 2));
    assert_eq!((el, ec), (3, 4));
    Ok(())
}

#[test]
fn req_range_returns_error_when_start_line_missing() {
    let params = json!({
        "range": {
            "start": { "character": 0 },
            "end": { "line": 1, "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("range.start.line"));
}

#[test]
fn req_range_returns_error_when_end_character_missing() {
    let params = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 1 }
        }
    });
    let result = req_range(&params);
    assert!(result.is_err());
    let err = result.err().unwrap_or_else(|| unreachable!());
    assert!(err.message.contains("range.end.character"));
}

#[test]
fn req_range_overflow_start_line() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({
        "range": {
            "start": { "line": over, "character": 0 },
            "end": { "line": 0, "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(result.is_err());
}

#[test]
fn req_range_overflow_end_line() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": over, "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(result.is_err());
}

#[test]
fn req_range_overflow_start_character() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({
        "range": {
            "start": { "line": 0, "character": over },
            "end": { "line": 0, "character": 0 }
        }
    });
    let result = req_range(&params);
    assert!(result.is_err());
}

#[test]
fn req_range_overflow_end_character() {
    let over = u64::from(u32::MAX) + 1;
    let params = json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": over }
        }
    });
    let result = req_range(&params);
    assert!(result.is_err());
}

#[test]
fn req_range_empty_params() {
    let result = req_range(&json!({}));
    assert!(result.is_err());
}

// ============================================================================
// Capabilities — capabilities_for with various BuildFlags profiles
// ============================================================================

#[test]
fn capabilities_for_production_has_core_features() {
    let caps =
        capabilities::capabilities_for(perl_lsp_rs_core::features::flags::BuildFlags::production());
    // Production profile enables the core advertised navigation/editing surface.
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_some());
    assert!(caps.completion_provider.is_some());
    assert!(caps.definition_provider.is_some());
    assert!(caps.references_provider.is_some());
    assert!(caps.document_symbol_provider.is_some());
    assert!(caps.folding_range_provider.is_some());
}

#[test]
fn capabilities_for_ga_lock_has_core_features() {
    let caps =
        capabilities::capabilities_for(perl_lsp_rs_core::features::flags::BuildFlags::ga_lock());
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_some());
    assert!(caps.completion_provider.is_some());
}

#[test]
fn capabilities_for_all_enables_conditional_features() {
    let flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    let caps = capabilities::capabilities_for(flags);
    assert!(caps.document_highlight_provider.is_some());
    assert!(caps.signature_help_provider.is_some());
    assert!(caps.declaration_provider.is_some());
    assert!(caps.type_definition_provider.is_some());
    assert!(caps.implementation_provider.is_some());
    assert!(caps.inlay_hint_provider.is_some());
    assert!(caps.rename_provider.is_some());
    assert!(caps.code_action_provider.is_some());
    assert!(caps.code_lens_provider.is_some());
    assert!(caps.document_link_provider.is_some());
    assert!(caps.selection_range_provider.is_some());
    assert!(caps.semantic_tokens_provider.is_some());
    assert!(caps.call_hierarchy_provider.is_some());
    assert!(caps.document_on_type_formatting_provider.is_some());
    assert!(caps.linked_editing_range_provider.is_some());
    assert!(caps.inline_value_provider.is_some());
    assert!(caps.moniker_provider.is_some());
    assert!(caps.color_provider.is_some());
    assert!(caps.diagnostic_provider.is_some());
    assert!(caps.document_formatting_provider.is_some());
    assert!(caps.document_range_formatting_provider.is_some());
}

#[test]
fn capabilities_for_minimal_flags_omits_conditional() {
    // Default derives all-false for bool fields
    let flags = perl_lsp_rs_core::features::flags::BuildFlags::default();
    let caps = capabilities::capabilities_for(flags);
    // Only text sync is unconditional; feature-gated providers should stay off.
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_none());
    assert!(caps.completion_provider.is_none());
    assert!(caps.definition_provider.is_none());
    assert!(caps.references_provider.is_none());
    assert!(caps.document_symbol_provider.is_none());
    assert!(caps.workspace_symbol_provider.is_none());
    assert!(caps.folding_range_provider.is_none());
    // Conditional should be off
    assert!(caps.document_highlight_provider.is_none());
    assert!(caps.signature_help_provider.is_none());
    assert!(caps.declaration_provider.is_none());
    assert!(caps.type_definition_provider.is_none());
    assert!(caps.implementation_provider.is_none());
    assert!(caps.inlay_hint_provider.is_none());
    assert!(caps.rename_provider.is_none());
    assert!(caps.code_action_provider.is_none());
    assert!(caps.code_lens_provider.is_none());
    assert!(caps.semantic_tokens_provider.is_none());
    assert!(caps.document_link_provider.is_none());
    assert!(caps.selection_range_provider.is_none());
    assert!(caps.document_on_type_formatting_provider.is_none());
    assert!(caps.linked_editing_range_provider.is_none());
    assert!(caps.inline_value_provider.is_none());
    assert!(caps.moniker_provider.is_none());
    assert!(caps.color_provider.is_none());
    assert!(caps.diagnostic_provider.is_none());
    assert!(caps.notebook_document_sync.is_none());
    assert!(caps.document_formatting_provider.is_none());
    assert!(caps.document_range_formatting_provider.is_none());
}

// ============================================================================
// Capabilities — capabilities_json
// ============================================================================

#[test]
fn capabilities_json_returns_valid_json() {
    let flags = perl_lsp_rs_core::features::flags::BuildFlags::production();
    let v = capabilities::capabilities_json(flags);
    assert!(v.is_object());
    assert!(v.get("hoverProvider").is_some());
}

#[test]
fn capabilities_json_includes_type_hierarchy_when_enabled() {
    let flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    let v = capabilities::capabilities_json(flags);
    assert!(v.get("typeHierarchyProvider").is_some());
}

#[test]
fn capabilities_json_omits_type_hierarchy_when_disabled() {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.type_hierarchy = false;
    let v = capabilities::capabilities_json(flags);
    assert!(v.get("typeHierarchyProvider").is_none());
}

// ============================================================================
// Capabilities — cap_bool_or_object
// ============================================================================

#[test]
fn cap_bool_or_object_true_for_boolean() {
    let caps = json!({"hoverProvider": true});
    assert!(capabilities::cap_bool_or_object(&caps, "hoverProvider"));
}

#[test]
fn cap_bool_or_object_true_for_object() {
    let caps = json!({"completionProvider": {"resolveProvider": true}});
    assert!(capabilities::cap_bool_or_object(&caps, "completionProvider"));
}

#[test]
fn cap_bool_or_object_false_for_missing() {
    let caps = json!({});
    assert!(!capabilities::cap_bool_or_object(&caps, "hoverProvider"));
}

#[test]
fn cap_bool_or_object_false_for_null() {
    let caps = json!({"hoverProvider": null});
    assert!(!capabilities::cap_bool_or_object(&caps, "hoverProvider"));
}

#[test]
fn cap_bool_or_object_false_for_string() {
    let caps = json!({"hoverProvider": "yes"});
    assert!(!capabilities::cap_bool_or_object(&caps, "hoverProvider"));
}

#[test]
fn cap_bool_or_object_false_for_number() {
    let caps = json!({"hoverProvider": 1});
    assert!(!capabilities::cap_bool_or_object(&caps, "hoverProvider"));
}

// ============================================================================
// Capabilities — get_supported_commands
// ============================================================================

#[test]
fn get_supported_commands_returns_nonempty() {
    let cmds = capabilities::get_supported_commands();
    assert!(!cmds.is_empty());
}

#[test]
fn get_supported_commands_all_start_with_perl() {
    let cmds = capabilities::get_supported_commands();
    for cmd in &cmds {
        assert!(cmd.starts_with("perl."), "Command {cmd} doesn't start with perl.");
    }
}

#[test]
fn get_supported_commands_contains_known_commands() {
    let cmds = capabilities::get_supported_commands();
    assert!(cmds.contains(&"perl.runTests".to_string()));
    assert!(cmds.contains(&"perl.runCritic".to_string()));
    assert!(cmds.contains(&"perl.runFile".to_string()));
    assert!(cmds.contains(&"perl.debugFile".to_string()));
}

#[test]
fn get_supported_commands_no_duplicates() {
    let cmds = capabilities::get_supported_commands();
    let mut seen = std::collections::HashSet::new();
    for cmd in &cmds {
        assert!(seen.insert(cmd), "Duplicate command: {cmd}");
    }
}

// ============================================================================
// Capabilities — default_capabilities
// ============================================================================

#[test]
fn default_capabilities_returns_valid_caps() {
    let caps = capabilities::default_capabilities();
    assert!(caps.text_document_sync.is_some());
    assert!(caps.hover_provider.is_some());
}

// ============================================================================
// Capabilities — inline_completion via experimental
// ============================================================================

#[test]
fn inline_completion_uses_experimental_field() {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.inline_completion = true;
    let caps = capabilities::capabilities_for(flags);
    let exp = caps.experimental.as_ref();
    assert!(exp.is_some());
    let exp = exp.unwrap_or_else(|| unreachable!());
    assert!(exp.get("inlineCompletionProvider").is_some());
}

#[test]
fn no_inline_completion_when_disabled() {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.inline_completion = false;
    let caps = capabilities::capabilities_for(flags);
    // experimental may still be None or may not have the key
    let has_inline =
        caps.experimental.as_ref().and_then(|e| e.get("inlineCompletionProvider")).is_some();
    assert!(!has_inline);
}

// ============================================================================
// Capabilities — notebook_document_sync
// ============================================================================

#[test]
fn notebook_sync_advertised_when_enabled() {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.notebook_document_sync = true;
    let caps = capabilities::capabilities_for(flags);
    assert!(caps.notebook_document_sync.is_some());
}

#[test]
fn notebook_sync_not_advertised_when_disabled() {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.notebook_document_sync = false;
    let caps = capabilities::capabilities_for(flags);
    assert!(caps.notebook_document_sync.is_none());
}

// ============================================================================
// Capabilities — code action kinds include REFACTOR_EXTRACT
// ============================================================================

#[test]
fn code_actions_include_refactor_extract() -> Result<(), Box<dyn std::error::Error>> {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.code_actions = true;
    let caps = capabilities::capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let kinds = v.pointer("/codeActionProvider/codeActionKinds").and_then(|v| v.as_array());
    assert!(kinds.is_some());
    let kinds = kinds.unwrap_or_else(|| unreachable!());
    let has_refactor = kinds.iter().any(|k| k.as_str() == Some("refactor.extract"));
    assert!(has_refactor, "codeActionKinds should include refactor.extract");
    Ok(())
}

#[test]
fn code_actions_include_source_organize_imports_when_enabled()
-> Result<(), Box<dyn std::error::Error>> {
    let mut flags = perl_lsp_rs_core::features::flags::BuildFlags::all();
    flags.code_actions = true;
    flags.source_organize_imports = true;
    let caps = capabilities::capabilities_for(flags);
    let v = serde_json::to_value(&caps)?;
    let kinds = v.pointer("/codeActionProvider/codeActionKinds").and_then(|v| v.as_array());
    assert!(kinds.is_some());
    let kinds = kinds.unwrap_or_else(|| unreachable!());
    let has_organize = kinds.iter().any(|k| k.as_str() == Some("source.organizeImports"));
    assert!(has_organize);
    Ok(())
}

// ============================================================================
// Method Constants — spot-check LSP 3.17 method names
// ============================================================================

#[test]
fn method_constants_lifecycle() {
    assert_eq!(methods::INITIALIZE, "initialize");
    assert_eq!(methods::INITIALIZED, "initialized");
    assert_eq!(methods::SHUTDOWN, "shutdown");
    assert_eq!(methods::EXIT, "exit");
}

#[test]
fn method_constants_text_document_sync() {
    assert_eq!(methods::TEXT_DOCUMENT_DID_OPEN, "textDocument/didOpen");
    assert_eq!(methods::TEXT_DOCUMENT_DID_CHANGE, "textDocument/didChange");
    assert_eq!(methods::TEXT_DOCUMENT_DID_CLOSE, "textDocument/didClose");
    assert_eq!(methods::TEXT_DOCUMENT_DID_SAVE, "textDocument/didSave");
    assert_eq!(methods::TEXT_DOCUMENT_WILL_SAVE, "textDocument/willSave");
    assert_eq!(methods::TEXT_DOCUMENT_WILL_SAVE_WAIT_UNTIL, "textDocument/willSaveWaitUntil");
    assert_eq!(methods::TEXT_DOCUMENT_PUBLISH_DIAGNOSTICS, "textDocument/publishDiagnostics");
}

#[test]
fn method_constants_completion() {
    assert_eq!(methods::TEXT_DOCUMENT_COMPLETION, "textDocument/completion");
    assert_eq!(methods::COMPLETION_ITEM_RESOLVE, "completionItem/resolve");
}

#[test]
fn method_constants_navigation() {
    assert_eq!(methods::TEXT_DOCUMENT_HOVER, "textDocument/hover");
    assert_eq!(methods::TEXT_DOCUMENT_SIGNATURE_HELP, "textDocument/signatureHelp");
    assert_eq!(methods::TEXT_DOCUMENT_DEFINITION, "textDocument/definition");
    assert_eq!(methods::TEXT_DOCUMENT_DECLARATION, "textDocument/declaration");
    assert_eq!(methods::TEXT_DOCUMENT_TYPE_DEFINITION, "textDocument/typeDefinition");
    assert_eq!(methods::TEXT_DOCUMENT_IMPLEMENTATION, "textDocument/implementation");
    assert_eq!(methods::TEXT_DOCUMENT_REFERENCES, "textDocument/references");
}

#[test]
fn method_constants_symbols() {
    assert_eq!(methods::TEXT_DOCUMENT_DOCUMENT_SYMBOL, "textDocument/documentSymbol");
    assert_eq!(methods::TEXT_DOCUMENT_DOCUMENT_HIGHLIGHT, "textDocument/documentHighlight");
}

#[test]
fn method_constants_code_actions() {
    assert_eq!(methods::TEXT_DOCUMENT_CODE_ACTION, "textDocument/codeAction");
    assert_eq!(methods::CODE_ACTION_RESOLVE, "codeAction/resolve");
    assert_eq!(methods::TEXT_DOCUMENT_CODE_LENS, "textDocument/codeLens");
    assert_eq!(methods::CODE_LENS_RESOLVE, "codeLens/resolve");
}

#[test]
fn method_constants_formatting() {
    assert_eq!(methods::TEXT_DOCUMENT_FORMATTING, "textDocument/formatting");
    assert_eq!(methods::TEXT_DOCUMENT_RANGE_FORMATTING, "textDocument/rangeFormatting");
    assert_eq!(methods::TEXT_DOCUMENT_RANGES_FORMATTING, "textDocument/rangesFormatting");
    assert_eq!(methods::TEXT_DOCUMENT_ON_TYPE_FORMATTING, "textDocument/onTypeFormatting");
}

#[test]
fn method_constants_rename() {
    assert_eq!(methods::TEXT_DOCUMENT_PREPARE_RENAME, "textDocument/prepareRename");
    assert_eq!(methods::TEXT_DOCUMENT_RENAME, "textDocument/rename");
    assert_eq!(methods::TEXT_DOCUMENT_LINKED_EDITING_RANGE, "textDocument/linkedEditingRange");
}

#[test]
fn method_constants_semantic_tokens() {
    assert_eq!(methods::TEXT_DOCUMENT_SEMANTIC_TOKENS_FULL, "textDocument/semanticTokens/full");
    assert_eq!(methods::TEXT_DOCUMENT_SEMANTIC_TOKENS_RANGE, "textDocument/semanticTokens/range");
}

#[test]
fn method_constants_inlay_hints() {
    assert_eq!(methods::TEXT_DOCUMENT_INLAY_HINT, "textDocument/inlayHint");
    assert_eq!(methods::INLAY_HINT_RESOLVE, "inlayHint/resolve");
}

#[test]
fn method_constants_document_links() {
    assert_eq!(methods::TEXT_DOCUMENT_DOCUMENT_LINK, "textDocument/documentLink");
    assert_eq!(methods::DOCUMENT_LINK_RESOLVE, "documentLink/resolve");
}

#[test]
fn method_constants_folding_and_selection() {
    assert_eq!(methods::TEXT_DOCUMENT_FOLDING_RANGE, "textDocument/foldingRange");
    assert_eq!(methods::TEXT_DOCUMENT_SELECTION_RANGE, "textDocument/selectionRange");
}

#[test]
fn method_constants_type_hierarchy() {
    assert_eq!(methods::TEXT_DOCUMENT_PREPARE_TYPE_HIERARCHY, "textDocument/prepareTypeHierarchy");
    assert_eq!(methods::TYPE_HIERARCHY_PREPARE, "typeHierarchy/prepare");
    assert_eq!(methods::TYPE_HIERARCHY_SUPERTYPES, "typeHierarchy/supertypes");
    assert_eq!(methods::TYPE_HIERARCHY_SUBTYPES, "typeHierarchy/subtypes");
}

#[test]
fn method_constants_call_hierarchy() {
    assert_eq!(methods::TEXT_DOCUMENT_PREPARE_CALL_HIERARCHY, "textDocument/prepareCallHierarchy");
    assert_eq!(methods::CALL_HIERARCHY_INCOMING_CALLS, "callHierarchy/incomingCalls");
    assert_eq!(methods::CALL_HIERARCHY_OUTGOING_CALLS, "callHierarchy/outgoingCalls");
}

#[test]
fn method_constants_diagnostics() {
    assert_eq!(methods::TEXT_DOCUMENT_DIAGNOSTIC, "textDocument/diagnostic");
    assert_eq!(methods::WORKSPACE_DIAGNOSTIC, "workspace/diagnostic");
}

#[test]
fn method_constants_inline_features() {
    assert_eq!(methods::TEXT_DOCUMENT_INLINE_COMPLETION, "textDocument/inlineCompletion");
    assert_eq!(methods::TEXT_DOCUMENT_INLINE_VALUE, "textDocument/inlineValue");
}

#[test]
fn method_constants_colors() {
    assert_eq!(methods::TEXT_DOCUMENT_DOCUMENT_COLOR, "textDocument/documentColor");
    assert_eq!(methods::TEXT_DOCUMENT_COLOR_PRESENTATION, "textDocument/colorPresentation");
}

#[test]
fn method_constants_moniker() {
    assert_eq!(methods::TEXT_DOCUMENT_MONIKER, "textDocument/moniker");
}

#[test]
fn method_constants_workspace() {
    assert_eq!(methods::WORKSPACE_SYMBOL, "workspace/symbol");
    assert_eq!(methods::WORKSPACE_SYMBOL_RESOLVE, "workspace/symbol/resolve");
    assert_eq!(methods::WORKSPACE_EXECUTE_COMMAND, "workspace/executeCommand");
    assert_eq!(methods::WORKSPACE_APPLY_EDIT, "workspace/applyEdit");
    assert_eq!(methods::WORKSPACE_CONFIGURATION, "workspace/configuration");
    assert_eq!(methods::WORKSPACE_TEXT_DOCUMENT_CONTENT, "workspace/textDocumentContent");
}

#[test]
fn method_constants_workspace_file_ops() {
    assert_eq!(methods::WORKSPACE_WILL_CREATE_FILES, "workspace/willCreateFiles");
    assert_eq!(methods::WORKSPACE_DID_CREATE_FILES, "workspace/didCreateFiles");
    assert_eq!(methods::WORKSPACE_WILL_RENAME_FILES, "workspace/willRenameFiles");
    assert_eq!(methods::WORKSPACE_DID_RENAME_FILES, "workspace/didRenameFiles");
    assert_eq!(methods::WORKSPACE_WILL_DELETE_FILES, "workspace/willDeleteFiles");
    assert_eq!(methods::WORKSPACE_DID_DELETE_FILES, "workspace/didDeleteFiles");
}

#[test]
fn method_constants_workspace_config() {
    assert_eq!(
        methods::WORKSPACE_DID_CHANGE_WORKSPACE_FOLDERS,
        "workspace/didChangeWorkspaceFolders"
    );
    assert_eq!(methods::WORKSPACE_DID_CHANGE_CONFIGURATION, "workspace/didChangeConfiguration");
    assert_eq!(methods::WORKSPACE_DID_CHANGE_WATCHED_FILES, "workspace/didChangeWatchedFiles");
}

#[test]
fn method_constants_workspace_refresh() {
    assert_eq!(methods::WORKSPACE_CODE_LENS_REFRESH, "workspace/codeLens/refresh");
    assert_eq!(methods::WORKSPACE_SEMANTIC_TOKENS_REFRESH, "workspace/semanticTokens/refresh");
    assert_eq!(methods::WORKSPACE_INLAY_HINT_REFRESH, "workspace/inlayHint/refresh");
    assert_eq!(methods::WORKSPACE_INLINE_VALUE_REFRESH, "workspace/inlineValue/refresh");
    assert_eq!(methods::WORKSPACE_DIAGNOSTIC_REFRESH, "workspace/diagnostic/refresh");
    assert_eq!(methods::WORKSPACE_FOLDING_RANGE_REFRESH, "workspace/foldingRange/refresh");
    assert_eq!(
        methods::WORKSPACE_TEXT_DOCUMENT_CONTENT_REFRESH,
        "workspace/textDocumentContent/refresh"
    );
}

#[test]
fn method_constants_notebook() {
    assert_eq!(methods::NOTEBOOK_DOCUMENT_DID_OPEN, "notebookDocument/didOpen");
    assert_eq!(methods::NOTEBOOK_DOCUMENT_DID_CHANGE, "notebookDocument/didChange");
    assert_eq!(methods::NOTEBOOK_DOCUMENT_DID_SAVE, "notebookDocument/didSave");
    assert_eq!(methods::NOTEBOOK_DOCUMENT_DID_CLOSE, "notebookDocument/didClose");
}

#[test]
fn method_constants_window() {
    assert_eq!(methods::WINDOW_SHOW_MESSAGE, "window/showMessage");
    assert_eq!(methods::WINDOW_LOG_MESSAGE, "window/logMessage");
    assert_eq!(methods::WINDOW_SHOW_MESSAGE_REQUEST, "window/showMessageRequest");
    assert_eq!(methods::WINDOW_SHOW_DOCUMENT, "window/showDocument");
    assert_eq!(methods::WINDOW_WORK_DONE_PROGRESS_CREATE, "window/workDoneProgress/create");
    assert_eq!(methods::WINDOW_WORK_DONE_PROGRESS_CANCEL, "window/workDoneProgress/cancel");
}

#[test]
fn method_constants_special() {
    assert_eq!(methods::CANCEL_REQUEST, "$/cancelRequest");
    assert_eq!(methods::DOLLAR_PROGRESS, "$/progress");
    assert_eq!(methods::TEST_SLOW_OPERATION, "$/test/slowOperation");
}

#[test]
fn method_constants_experimental() {
    assert_eq!(methods::EXPERIMENTAL_TEST_DISCOVERY, "experimental/testDiscovery");
}

// ============================================================================
// Round-trip: request → response cycle
// ============================================================================

#[test]
fn roundtrip_request_to_success_response() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "textDocument/hover",
        "params": { "textDocument": { "uri": "file:///x.pl" }, "position": { "line": 0, "character": 0 } }
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;

    let resp = JsonRpcResponse::success(req.id.clone(), json!({"contents": "sub foo"}));
    let v = serde_json::to_value(&resp)?;
    assert_eq!(v["id"], 100);
    assert_eq!(v["result"]["contents"], "sub foo");
    Ok(())
}

#[test]
fn roundtrip_request_to_error_response() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 200,
        "method": "textDocument/unknown"
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;

    let err = method_not_found(&req.method);
    let resp = JsonRpcResponse::error(req.id.clone(), err);
    let v = serde_json::to_value(&resp)?;
    assert_eq!(v["id"], 200);
    assert_eq!(v["error"]["code"], METHOD_NOT_FOUND);
    assert!(v["error"]["message"].as_str().is_some_and(|s| s.contains("textDocument/unknown")));
    Ok(())
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn request_with_complex_params() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "textDocument/completion",
        "params": {
            "textDocument": { "uri": "file:///test.pl" },
            "position": { "line": 42, "character": 7 },
            "context": { "triggerKind": 1, "triggerCharacter": "$" }
        }
    });
    let req: JsonRpcRequest = serde_json::from_value(raw)?;
    let params = req.params.as_ref();
    assert!(params.is_some());
    let params = params.unwrap_or_else(|| unreachable!());
    assert_eq!(params["context"]["triggerCharacter"], "$");
    Ok(())
}

#[test]
fn response_with_large_result() -> Result<(), Box<dyn std::error::Error>> {
    let items: Vec<Value> = (0..1000).map(|i| json!({"label": format!("item_{i}")})).collect();
    let resp = JsonRpcResponse::success(Some(json!(1)), json!(items));
    let v = serde_json::to_value(&resp)?;
    let arr = v["result"].as_array();
    assert!(arr.is_some());
    let arr = arr.unwrap_or_else(|| unreachable!());
    assert_eq!(arr.len(), 1000);
    Ok(())
}

#[test]
fn error_with_into_string_accepts_string_ref() {
    let err = JsonRpcError::new(-1, "hello");
    assert_eq!(err.message, "hello");
}

#[test]
fn error_with_into_string_accepts_owned_string() {
    let msg = String::from("owned message");
    let err = JsonRpcError::new(-1, msg);
    assert_eq!(err.message, "owned message");
}
