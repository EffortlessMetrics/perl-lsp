//! JSON-RPC error codes and error response builders
//!
//! Standard JSON-RPC 2.0 error codes plus LSP-specific extensions.

use super::jsonrpc::{JsonRpcError, JsonRpcResponse};
use serde_json::{Value, json};

/// Typed error code enum for LSP/JSON-RPC error codes.
///
/// Provides a typed interface over the raw `i32` error code constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    /// Parse error — invalid JSON received.
    ParseError = -32700,
    /// Invalid request — the JSON is not a valid request.
    InvalidRequest = -32600,
    /// Method not found.
    MethodNotFound = -32601,
    /// Invalid params.
    InvalidParams = -32602,
    /// Internal error.
    InternalError = -32603,
    /// Server error start boundary.
    ServerErrorStart = -32099,
    /// Server error end boundary.
    ServerErrorEnd = -32000,
    /// Request cancelled (LSP 3.17).
    RequestCancelled = -32800,
    /// Server cancelled (LSP 3.17).
    ServerCancelled = -32802,
    /// Content modified.
    ContentModified = -32801,
    /// Request failed (LSP 3.17).
    RequestFailed = -32803,
    /// Server not initialized.
    ServerNotInitialized = -32002,
}

// ============================================================================
// JSON-RPC 2.0 Standard Error Codes
// ============================================================================

/// Parse error - Invalid JSON was received
pub const PARSE_ERROR: i32 = -32700;

/// Invalid Request - The JSON sent is not a valid Request object
pub const INVALID_REQUEST: i32 = -32600;

/// Method not found - The method does not exist / is not available
pub const METHOD_NOT_FOUND: i32 = -32601;

/// Invalid params - Invalid method parameter(s)
pub const INVALID_PARAMS: i32 = -32602;

/// Internal error - Internal JSON-RPC error
pub const INTERNAL_ERROR: i32 = -32603;

// ============================================================================
// JSON-RPC Reserved Error Code Ranges
// ============================================================================

/// Server error range start (reserved for implementation-defined server-errors)
/// Per JSON-RPC 2.0 spec, server errors are between -32099 and -32000 inclusive.
pub const SERVER_ERROR_START: i32 = -32099;

/// Server error range end (inclusive)
/// Per JSON-RPC 2.0 spec, server errors are between -32099 and -32000 inclusive.
pub const SERVER_ERROR_END: i32 = -32000;

/// Unknown error code (for internal use)
pub const UNKNOWN_ERROR_CODE: i32 = -32001;

/// Connection closed - The connection was closed unexpectedly
///
/// Used when a BrokenPipe or similar transport error indicates
/// the client/server connection has been terminated.
/// Reserved server error range: -32000 to -32099
pub const CONNECTION_CLOSED: i32 = -32050;

/// Transport error - A general transport-layer error occurred
///
/// Used for I/O errors that are not specifically connection closures,
/// such as write failures, buffer overflows, etc.
/// Reserved server error range: -32000 to -32099
pub const TRANSPORT_ERROR: i32 = -32051;

// ============================================================================
// LSP 3.17 Standard Error Codes
// ============================================================================

/// Server cancelled the request (LSP 3.17)
///
/// Used when the server decides to cancel an in-flight request,
/// typically due to resource constraints or newer conflicting requests.
pub const SERVER_CANCELLED: i32 = -32802;

/// Content modified - The document content was modified during operation
///
/// Indicates the operation was obsoleted by document changes.
pub const CONTENT_MODIFIED: i32 = -32801;

/// Request cancelled - Client cancelled via $/cancelRequest
///
/// Used when responding to a request that was explicitly cancelled
/// by the client through the $/cancelRequest notification.
pub const REQUEST_CANCELLED: i32 = -32800;

/// Request failed - Generic request failure (LSP 3.17)
pub const REQUEST_FAILED: i32 = -32803;

// ============================================================================
// LSP-Specific Error Codes
// ============================================================================

/// Server not initialized
///
/// Per LSP spec, requests (other than initialize) received before
/// the server is initialized should return this error.
pub const SERVER_NOT_INITIALIZED: i32 = -32002;

// ============================================================================
// Error Response Builders
// ============================================================================

/// Create a standard cancelled response
pub fn cancelled_response(id: &Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(id.clone()),
        result: None,
        error: Some(JsonRpcError {
            code: REQUEST_CANCELLED,
            message: "Request cancelled".into(),
            data: None,
        }),
    }
}

/// Create a cancelled response with method/provider context
///
/// This enhanced version includes the provider name in the error message and data,
/// allowing clients to track which specific operation was cancelled.
pub fn cancelled_response_with_method(id: &Value, method: &str) -> JsonRpcResponse {
    // Extract the short provider name from the full method path
    let provider_name = method.split('/').next_back().unwrap_or(method);
    let message = format!("Request cancelled - {} provider", provider_name);

    let data = json!({
        "provider": method,
        "request_id": id.clone(),
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    });

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(id.clone()),
        result: None,
        error: Some(JsonRpcError { code: REQUEST_CANCELLED, message, data: Some(data) }),
    }
}

/// Create a request cancelled error
pub fn request_cancelled_error() -> JsonRpcError {
    JsonRpcError { code: REQUEST_CANCELLED, message: "Request cancelled".to_string(), data: None }
}

/// Create a server cancelled error
pub fn server_cancelled_error() -> JsonRpcError {
    JsonRpcError {
        code: SERVER_CANCELLED,
        message: "Server cancelled the request".to_string(),
        data: None,
    }
}

/// Create an enhanced error response with comprehensive context
pub fn enhanced_error(
    code: i32,
    message: &str,
    error_type: &str,
    method: Option<&str>,
) -> JsonRpcError {
    let mut data = json!({
        "error_type": error_type,
        "context": "Enhanced LSP error response with comprehensive context",
        "server_info": {
            "name": "perl-lsp",
            "version": env!("CARGO_PKG_VERSION"),
            "capabilities": "Enhanced error handling and concurrent request management"
        },
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    });

    if let Some(method_name) = method {
        data["method"] = json!(method_name);
    }

    JsonRpcError { code, message: message.to_string(), data: Some(data) }
}

/// Create a method not found error
pub fn method_not_found(method: &str) -> JsonRpcError {
    JsonRpcError {
        code: METHOD_NOT_FOUND,
        message: format!("Method not found: {}", method),
        data: None,
    }
}

/// Create a method not advertised error
///
/// Used when the client requests a feature that wasn't advertised
/// in the server's capabilities during initialization.
pub fn method_not_advertised() -> JsonRpcError {
    JsonRpcError {
        code: METHOD_NOT_FOUND,
        message: "Method not advertised in server capabilities".to_string(),
        data: None,
    }
}

/// Create an invalid params error
pub fn invalid_params(message: &str) -> JsonRpcError {
    JsonRpcError { code: INVALID_PARAMS, message: message.to_string(), data: None }
}

/// Create a server not initialized error
pub fn server_not_initialized() -> JsonRpcError {
    JsonRpcError {
        code: SERVER_NOT_INITIALIZED,
        message: "Server not initialized".to_string(),
        data: None,
    }
}

/// Create a document not found error response value
pub fn document_not_found_error() -> Value {
    json!({
        "status": "error",
        "message": "Document not found"
    })
}

/// Create an internal error
pub fn internal_error(message: &str) -> JsonRpcError {
    JsonRpcError { code: INTERNAL_ERROR, message: message.to_string(), data: None }
}

/// Create a connection closed error
///
/// Used when the connection to the client has been terminated (e.g., BrokenPipe).
/// This is a transport-layer error, distinct from protocol-level InvalidRequest.
pub fn connection_closed_error() -> JsonRpcError {
    JsonRpcError { code: CONNECTION_CLOSED, message: "Connection closed".to_string(), data: None }
}

/// Create a transport error with custom message
///
/// Used for general I/O/transport errors that aren't specifically connection closures.
pub fn transport_error(message: &str) -> JsonRpcError {
    JsonRpcError { code: TRANSPORT_ERROR, message: message.to_string(), data: None }
}

// ============================================================================
// Request Parameter Extraction Helpers
// ============================================================================

/// Extract the required textDocument.uri from LSP request params
///
/// Returns INVALID_PARAMS error if the URI is missing or not a string.
pub fn req_uri(params: &Value) -> Result<&str, JsonRpcError> {
    params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid_params("Missing required parameter: textDocument.uri"))
}

/// Extract the required position (line, character) from LSP request params
///
/// Returns INVALID_PARAMS error if line or character are missing or overflow u32.
pub fn req_position(params: &Value) -> Result<(u32, u32), JsonRpcError> {
    let line_u64 = params
        .pointer("/position/line")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| invalid_params("Missing required parameter: position.line"))?;
    let line =
        u32::try_from(line_u64).map_err(|_| invalid_params("position.line exceeds u32::MAX"))?;
    let character_u64 = params
        .pointer("/position/character")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| invalid_params("Missing required parameter: position.character"))?;
    let character = u32::try_from(character_u64)
        .map_err(|_| invalid_params("position.character exceeds u32::MAX"))?;
    Ok((line, character))
}

/// Extract the required range from LSP request params
///
/// Returns INVALID_PARAMS error if any range components are missing or overflow u32.
/// Returns ((start_line, start_char), (end_line, end_char)).
pub fn req_range(params: &Value) -> Result<((u32, u32), (u32, u32)), JsonRpcError> {
    let start_line_u64 = params
        .pointer("/range/start/line")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| invalid_params("Missing required parameter: range.start.line"))?;
    let start_line = u32::try_from(start_line_u64)
        .map_err(|_| invalid_params("range.start.line exceeds u32::MAX"))?;
    let start_char_u64 = params
        .pointer("/range/start/character")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| invalid_params("Missing required parameter: range.start.character"))?;
    let start_char = u32::try_from(start_char_u64)
        .map_err(|_| invalid_params("range.start.character exceeds u32::MAX"))?;
    let end_line_u64 = params
        .pointer("/range/end/line")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| invalid_params("Missing required parameter: range.end.line"))?;
    let end_line = u32::try_from(end_line_u64)
        .map_err(|_| invalid_params("range.end.line exceeds u32::MAX"))?;
    let end_char_u64 = params
        .pointer("/range/end/character")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| invalid_params("Missing required parameter: range.end.character"))?;
    let end_char = u32::try_from(end_char_u64)
        .map_err(|_| invalid_params("range.end.character exceeds u32::MAX"))?;
    Ok(((start_line, start_char), (end_line, end_char)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::error::Error;

    // =========================================================================
    // ErrorCode enum — verify repr values match constants
    // =========================================================================

    #[test]
    fn error_code_parse_error_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::ParseError as i32, PARSE_ERROR);
        assert_eq!(PARSE_ERROR, -32700);
        Ok(())
    }

    #[test]
    fn error_code_invalid_request_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::InvalidRequest as i32, INVALID_REQUEST);
        assert_eq!(INVALID_REQUEST, -32600);
        Ok(())
    }

    #[test]
    fn error_code_method_not_found_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::MethodNotFound as i32, METHOD_NOT_FOUND);
        assert_eq!(METHOD_NOT_FOUND, -32601);
        Ok(())
    }

    #[test]
    fn error_code_invalid_params_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::InvalidParams as i32, INVALID_PARAMS);
        assert_eq!(INVALID_PARAMS, -32602);
        Ok(())
    }

    #[test]
    fn error_code_internal_error_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::InternalError as i32, INTERNAL_ERROR);
        assert_eq!(INTERNAL_ERROR, -32603);
        Ok(())
    }

    #[test]
    fn error_code_server_error_start_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::ServerErrorStart as i32, SERVER_ERROR_START);
        assert_eq!(SERVER_ERROR_START, -32099);
        Ok(())
    }

    #[test]
    fn error_code_server_error_end_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::ServerErrorEnd as i32, SERVER_ERROR_END);
        assert_eq!(SERVER_ERROR_END, -32000);
        Ok(())
    }

    #[test]
    fn error_code_request_cancelled_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::RequestCancelled as i32, REQUEST_CANCELLED);
        assert_eq!(REQUEST_CANCELLED, -32800);
        Ok(())
    }

    #[test]
    fn error_code_server_cancelled_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::ServerCancelled as i32, SERVER_CANCELLED);
        assert_eq!(SERVER_CANCELLED, -32802);
        Ok(())
    }

    #[test]
    fn error_code_content_modified_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::ContentModified as i32, CONTENT_MODIFIED);
        assert_eq!(CONTENT_MODIFIED, -32801);
        Ok(())
    }

    #[test]
    fn error_code_request_failed_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::RequestFailed as i32, REQUEST_FAILED);
        assert_eq!(REQUEST_FAILED, -32803);
        Ok(())
    }

    #[test]
    fn error_code_server_not_initialized_matches_constant() -> Result<(), Box<dyn Error>> {
        assert_eq!(ErrorCode::ServerNotInitialized as i32, SERVER_NOT_INITIALIZED);
        assert_eq!(SERVER_NOT_INITIALIZED, -32002);
        Ok(())
    }

    // =========================================================================
    // Error builders — code, message, data shape
    // =========================================================================

    #[test]
    fn request_cancelled_error_has_correct_code_and_message() -> Result<(), Box<dyn Error>> {
        let e = request_cancelled_error();
        assert_eq!(e.code, REQUEST_CANCELLED);
        assert_eq!(e.message, "Request cancelled");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn server_cancelled_error_has_correct_code_and_message_starts_with()
    -> Result<(), Box<dyn Error>> {
        let e = server_cancelled_error();
        assert_eq!(e.code, SERVER_CANCELLED);
        assert!(e.message.starts_with("Server cancelled"), "message was: {}", e.message);
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn method_not_found_contains_method_name() -> Result<(), Box<dyn Error>> {
        let e = method_not_found("foo/bar");
        assert_eq!(e.code, METHOD_NOT_FOUND);
        assert!(e.message.contains("foo/bar"), "message was: {}", e.message);
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn method_not_advertised_has_correct_code_and_specific_message() -> Result<(), Box<dyn Error>> {
        let e = method_not_advertised();
        assert_eq!(e.code, METHOD_NOT_FOUND);
        assert_eq!(e.message, "Method not advertised in server capabilities");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn invalid_params_sets_code_and_message() -> Result<(), Box<dyn Error>> {
        let e = invalid_params("bad x");
        assert_eq!(e.code, INVALID_PARAMS);
        assert_eq!(e.message, "bad x");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn server_not_initialized_has_correct_code_and_message() -> Result<(), Box<dyn Error>> {
        let e = server_not_initialized();
        assert_eq!(e.code, SERVER_NOT_INITIALIZED);
        assert_eq!(e.message, "Server not initialized");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn internal_error_sets_code_and_message() -> Result<(), Box<dyn Error>> {
        let e = internal_error("oops");
        assert_eq!(e.code, INTERNAL_ERROR);
        assert_eq!(e.message, "oops");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn connection_closed_error_has_correct_code_and_message() -> Result<(), Box<dyn Error>> {
        let e = connection_closed_error();
        assert_eq!(e.code, CONNECTION_CLOSED);
        assert_eq!(e.message, "Connection closed");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn transport_error_sets_code_and_message() -> Result<(), Box<dyn Error>> {
        let e = transport_error("io");
        assert_eq!(e.code, TRANSPORT_ERROR);
        assert_eq!(e.message, "io");
        assert!(e.data.is_none());
        Ok(())
    }

    #[test]
    fn document_not_found_error_returns_value_with_status_and_message() -> Result<(), Box<dyn Error>>
    {
        let v = document_not_found_error();
        assert_eq!(v["status"], json!("error"));
        assert_eq!(v["message"], json!("Document not found"));
        Ok(())
    }

    // =========================================================================
    // Response builders
    // =========================================================================

    #[test]
    fn cancelled_response_has_correct_id_and_error_code() -> Result<(), Box<dyn Error>> {
        let resp = cancelled_response(&json!(7));
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(json!(7)));
        assert!(resp.result.is_none());
        let error = resp.error.ok_or("expected error field")?;
        assert_eq!(error.code, REQUEST_CANCELLED);
        assert!(error.data.is_none());
        Ok(())
    }

    #[test]
    fn cancelled_response_with_method_namespaced_sets_id_code_and_data()
    -> Result<(), Box<dyn Error>> {
        let resp = cancelled_response_with_method(&json!(42), "textDocument/hover");
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(json!(42)));
        assert!(resp.result.is_none());
        let error = resp.error.ok_or("expected error field")?;
        assert_eq!(error.code, REQUEST_CANCELLED);
        assert!(
            error.message.contains("hover"),
            "expected 'hover' in message, got: {}",
            error.message
        );
        let data = error.data.ok_or("expected data field")?;
        assert_eq!(data["provider"], json!("textDocument/hover"));
        assert_eq!(data["request_id"], json!(42));
        Ok(())
    }

    #[test]
    fn cancelled_response_with_method_no_slash_uses_method_as_provider()
    -> Result<(), Box<dyn Error>> {
        // Covers the `unwrap_or(method)` branch when there is no '/' in the method name.
        let resp = cancelled_response_with_method(&json!(1), "plain");
        let error = resp.error.ok_or("expected error field")?;
        assert_eq!(error.code, REQUEST_CANCELLED);
        // provider_name falls back to the full method name "plain"
        assert!(
            error.message.contains("plain"),
            "expected 'plain' in message, got: {}",
            error.message
        );
        let data = error.data.ok_or("expected data field")?;
        assert_eq!(data["provider"], json!("plain"));
        Ok(())
    }

    // =========================================================================
    // enhanced_error — data payload shape
    // =========================================================================

    #[test]
    fn enhanced_error_with_method_has_all_fields() -> Result<(), Box<dyn Error>> {
        let e = enhanced_error(-100, "msg", "etype", Some("textDocument/hover"));
        assert_eq!(e.code, -100);
        assert_eq!(e.message, "msg");
        let data = e.data.ok_or("expected data")?;
        assert_eq!(data["error_type"], json!("etype"));
        assert_eq!(data["server_info"]["name"], json!("perl-lsp"));
        assert_eq!(data["method"], json!("textDocument/hover"));
        Ok(())
    }

    #[test]
    fn enhanced_error_without_method_omits_method_field() -> Result<(), Box<dyn Error>> {
        let e = enhanced_error(-100, "msg", "etype", None);
        assert_eq!(e.code, -100);
        let data = e.data.ok_or("expected data")?;
        assert_eq!(data["error_type"], json!("etype"));
        // The `if let Some` branch is not taken — method key must be absent.
        assert!(data.get("method").is_none(), "method should not be present");
        Ok(())
    }

    // =========================================================================
    // req_uri — happy path and error cases
    // =========================================================================

    #[test]
    fn req_uri_happy_path() -> Result<(), Box<dyn Error>> {
        let params = json!({"textDocument": {"uri": "file:///a"}});
        let uri = req_uri(&params)?;
        assert_eq!(uri, "file:///a");
        Ok(())
    }

    #[test]
    fn req_uri_missing_text_document_returns_invalid_params() -> Result<(), Box<dyn Error>> {
        let Err(e) = req_uri(&json!({})) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("textDocument.uri"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_uri_non_string_uri_returns_invalid_params() -> Result<(), Box<dyn Error>> {
        let params = json!({"textDocument": {"uri": 42}});
        let Err(e) = req_uri(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        Ok(())
    }

    // =========================================================================
    // req_position — happy path, missing fields, u32 overflow
    // =========================================================================

    #[test]
    fn req_position_happy_path() -> Result<(), Box<dyn Error>> {
        let params = json!({"position": {"line": 1, "character": 2}});
        let (line, ch) = req_position(&params)?;
        assert_eq!(line, 1);
        assert_eq!(ch, 2);
        Ok(())
    }

    #[test]
    fn req_position_missing_position_returns_err_with_line_msg() -> Result<(), Box<dyn Error>> {
        let Err(e) = req_position(&json!({})) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("position.line"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_position_missing_character_returns_err_with_character_msg() -> Result<(), Box<dyn Error>>
    {
        let params = json!({"position": {"line": 1}});
        let Err(e) = req_position(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("position.character"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_position_line_overflow_u32_returns_err() -> Result<(), Box<dyn Error>> {
        let big: u64 = u64::from(u32::MAX) + 1;
        let params = json!({"position": {"line": big, "character": 2}});
        let Err(e) = req_position(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("exceeds u32::MAX"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_position_character_overflow_u32_returns_err() -> Result<(), Box<dyn Error>> {
        let big: u64 = u64::from(u32::MAX) + 1;
        let params = json!({"position": {"line": 1, "character": big}});
        let Err(e) = req_position(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("exceeds u32::MAX"), "message was: {}", e.message);
        Ok(())
    }

    // =========================================================================
    // req_range — happy path, missing leaves, u32 overflow on all four leaves
    // =========================================================================

    #[test]
    fn req_range_happy_path() -> Result<(), Box<dyn Error>> {
        let params = json!({
            "range": {
                "start": {"line": 0, "character": 1},
                "end":   {"line": 2, "character": 3}
            }
        });
        let ((sl, sc), (el, ec)) = req_range(&params)?;
        assert_eq!(sl, 0);
        assert_eq!(sc, 1);
        assert_eq!(el, 2);
        assert_eq!(ec, 3);
        Ok(())
    }

    #[test]
    fn req_range_missing_start_line_returns_err() -> Result<(), Box<dyn Error>> {
        let params = json!({
            "range": {
                "start": {"character": 1},
                "end":   {"line": 2, "character": 3}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("range.start.line"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_missing_start_character_returns_err() -> Result<(), Box<dyn Error>> {
        let params = json!({
            "range": {
                "start": {"line": 0},
                "end":   {"line": 2, "character": 3}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("range.start.character"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_missing_end_line_returns_err() -> Result<(), Box<dyn Error>> {
        let params = json!({
            "range": {
                "start": {"line": 0, "character": 1},
                "end":   {"character": 3}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("range.end.line"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_missing_end_character_returns_err() -> Result<(), Box<dyn Error>> {
        let params = json!({
            "range": {
                "start": {"line": 0, "character": 1},
                "end":   {"line": 2}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("range.end.character"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_start_line_overflow_u32_returns_err() -> Result<(), Box<dyn Error>> {
        let big: u64 = u64::from(u32::MAX) + 1;
        let params = json!({
            "range": {
                "start": {"line": big, "character": 1},
                "end":   {"line": 2, "character": 3}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("exceeds u32::MAX"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_start_character_overflow_u32_returns_err() -> Result<(), Box<dyn Error>> {
        let big: u64 = u64::from(u32::MAX) + 1;
        let params = json!({
            "range": {
                "start": {"line": 0, "character": big},
                "end":   {"line": 2, "character": 3}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("exceeds u32::MAX"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_end_line_overflow_u32_returns_err() -> Result<(), Box<dyn Error>> {
        let big: u64 = u64::from(u32::MAX) + 1;
        let params = json!({
            "range": {
                "start": {"line": 0, "character": 1},
                "end":   {"line": big, "character": 3}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("exceeds u32::MAX"), "message was: {}", e.message);
        Ok(())
    }

    #[test]
    fn req_range_end_character_overflow_u32_returns_err() -> Result<(), Box<dyn Error>> {
        let big: u64 = u64::from(u32::MAX) + 1;
        let params = json!({
            "range": {
                "start": {"line": 0, "character": 1},
                "end":   {"line": 2, "character": big}
            }
        });
        let Err(e) = req_range(&params) else {
            return Err("expected Err".into());
        };
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("exceeds u32::MAX"), "message was: {}", e.message);
        Ok(())
    }
}
