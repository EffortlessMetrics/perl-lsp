//! LSP Error helpers
//!
//! Provides consistent error responses for the LSP server.
//!
//! This module re-exports error codes and helper functions from
//! `crate::lsp::protocol` for backward compatibility.

// Re-export error codes from the canonical location
pub mod error_codes {
    //! LSP error codes (from the LSP 3.18 specification)
    pub use crate::lsp::protocol::{
        CONTENT_MODIFIED, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND,
        PARSE_ERROR, REQUEST_CANCELLED, REQUEST_FAILED, SERVER_CANCELLED, SERVER_ERROR_END,
        SERVER_ERROR_START, SERVER_NOT_INITIALIZED, UNKNOWN_ERROR_CODE,
    };
}

// Re-export helper functions from the canonical location
pub use crate::lsp::protocol::{
    cancelled_response, document_not_found_error, enhanced_error, internal_error, invalid_params,
    method_not_advertised, method_not_found, request_cancelled_error, server_cancelled_error,
    server_not_initialized,
};

// For backward compatibility, also provide these aliases
use crate::lsp::protocol::JsonRpcError;
use serde_json::{Value, json};

/// Create a server cancelled error with a custom message
///
/// This variant allows a custom message, unlike `server_cancelled_error()` which
/// uses a default message.
pub fn server_cancelled(message: &str) -> JsonRpcError {
    JsonRpcError { code: error_codes::SERVER_CANCELLED, message: message.to_string(), data: None }
}

/// Create an invalid params error as a JSON Value
///
/// This returns a JSON Value rather than JsonRpcError for legacy compatibility.
pub fn invalid_params_value(message: &str) -> Value {
    json!({
        "code": error_codes::INVALID_PARAMS,
        "message": message
    })
}

/// Create an internal error as a JSON Value
///
/// This returns a JSON Value rather than JsonRpcError for legacy compatibility.
pub fn internal_error_value(message: &str) -> Value {
    json!({
        "code": error_codes::INTERNAL_ERROR,
        "message": message
    })
}

/// Alias for invalid_params returning JsonRpcError
pub fn invalid_params_err(message: &str) -> JsonRpcError {
    invalid_params(message)
}

/// Alias for internal_error returning JsonRpcError
pub fn internal_error_err(message: &str) -> JsonRpcError {
    internal_error(message)
}

// Note: AdvertisedFeatures is defined in crate::capabilities, not here.
// For backward compatibility, re-export it:
pub use crate::capabilities::AdvertisedFeatures;
