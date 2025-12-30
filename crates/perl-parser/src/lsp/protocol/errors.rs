//! JSON-RPC error codes and error response builders
//!
//! Standard JSON-RPC 2.0 error codes plus LSP-specific extensions.

use super::jsonrpc::{JsonRpcError, JsonRpcResponse};
use serde_json::{Value, json};

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
