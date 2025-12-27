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
pub const SERVER_ERROR_START: i32 = -32099;

/// Server error range end
pub const SERVER_ERROR_END: i32 = -32000;

/// Unknown error code (for internal use)
pub const UNKNOWN_ERROR_CODE: i32 = -32001;

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
    JsonRpcError {
        code: REQUEST_CANCELLED,
        message: "Request cancelled".to_string(),
        data: None,
    }
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
    JsonRpcError {
        code: INVALID_PARAMS,
        message: message.to_string(),
        data: None,
    }
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
    JsonRpcError {
        code: INTERNAL_ERROR,
        message: message.to_string(),
        data: None,
    }
}
