//! JSON-RPC 2.0 message types
//!
//! Core request, response, and error types for JSON-RPC communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request message
///
/// Represents an incoming request from the LSP client.
/// The `id` field is `None` for notifications.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    #[serde(rename = "jsonrpc")]
    pub _jsonrpc: String,

    /// Request identifier (None for notifications)
    pub id: Option<Value>,

    /// Method name to invoke
    pub method: String,

    /// Method parameters
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response message
///
/// Represents an outgoing response to the LSP client.
/// Either `result` or `error` should be set, but not both.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Request identifier (matches the request's id)
    pub id: Option<Value>,

    /// Success result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error result (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
    }

    /// Create an error response
    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: None, error: Some(error) }
    }

    /// Create a null result response (for methods that return nothing)
    pub fn null(id: Option<Value>) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(Value::Null), error: None }
    }
}

/// JSON-RPC 2.0 error object
///
/// Represents an error that occurred during request processing.
#[derive(Debug, Serialize, Clone)]
pub struct JsonRpcError {
    /// Error code (see protocol/errors.rs for standard codes)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error data (optional)
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), data: None }
    }

    /// Create an error with additional data
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self { code, message: message.into(), data: Some(data) }
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}
