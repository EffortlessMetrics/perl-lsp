//! JSON-RPC 2.0 message types
//!
//! Core request, response, and error types for JSON-RPC communication.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// JSON-RPC request/response id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// Numeric JSON-RPC id.
    Integer(i64),
    /// String JSON-RPC id.
    String(String),
}

impl JsonRpcId {
    /// Convert a raw JSON value into a valid JSON-RPC id.
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Number(number) => number.as_i64().map(Self::Integer),
            Value::String(value) => Some(Self::String(value.clone())),
            _ => None,
        }
    }

    /// Convert this id back into a JSON value for legacy internals.
    pub fn to_value(&self) -> Value {
        match self {
            Self::Integer(value) => Value::Number((*value).into()),
            Self::String(value) => Value::String(value.clone()),
        }
    }
}

impl<'de> Deserialize<'de> for JsonRpcId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Number(number) => number
                .as_i64()
                .map(JsonRpcId::Integer)
                .ok_or_else(|| serde::de::Error::custom("JSON-RPC id must be an integer")),
            Value::String(value) => Ok(JsonRpcId::String(value)),
            _ => Err(serde::de::Error::custom("JSON-RPC id must be an integer or string")),
        }
    }
}

fn deserialize_optional_json_rpc_id<'de, D>(deserializer: D) -> Result<Option<JsonRpcId>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_i64()
            .map(JsonRpcId::Integer)
            .ok_or_else(|| serde::de::Error::custom("JSON-RPC id must be an integer"))
            .map(Some),
        Value::String(value) => Ok(Some(JsonRpcId::String(value))),
        Value::Null => Err(serde::de::Error::custom("JSON-RPC id must not be null")),
        _ => Err(serde::de::Error::custom("JSON-RPC id must be an integer or string")),
    }
}

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
    #[serde(default, deserialize_with = "deserialize_optional_json_rpc_id")]
    pub id: Option<JsonRpcId>,

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
    pub id: Option<JsonRpcId>,

    /// Success result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error result (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Option<JsonRpcId>, result: Value) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: Some(result), error: None }
    }

    /// Create an error response
    pub fn error(id: Option<JsonRpcId>, error: JsonRpcError) -> Self {
        Self { jsonrpc: "2.0".to_string(), id, result: None, error: Some(error) }
    }

    /// Create a null result response (for methods that return nothing)
    pub fn null(id: Option<JsonRpcId>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn json_rpc_request_accepts_integer_id() -> Result<(), Box<dyn Error>> {
        let request: JsonRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"method":"x"}"#)?;
        assert_eq!(request.id, Some(JsonRpcId::Integer(1)));
        Ok(())
    }

    #[test]
    fn json_rpc_request_accepts_string_id() -> Result<(), Box<dyn Error>> {
        let request: JsonRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":"abc-123","method":"x"}"#)?;
        assert_eq!(request.id, Some(JsonRpcId::String("abc-123".to_string())));
        Ok(())
    }

    #[test]
    fn json_rpc_response_echoes_string_id() -> Result<(), Box<dyn Error>> {
        let response =
            JsonRpcResponse::success(Some(JsonRpcId::String("abc-123".to_string())), Value::Null);
        let serialized = serde_json::to_value(response)?;
        assert_eq!(serialized["id"], Value::String("abc-123".to_string()));
        Ok(())
    }

    #[test]
    fn json_rpc_rejects_null_id_for_request() {
        let request =
            serde_json::from_str::<JsonRpcRequest>(r#"{"jsonrpc":"2.0","id":null,"method":"x"}"#);
        assert!(request.is_err());
    }

    #[test]
    fn json_rpc_rejects_fractional_id() {
        let request =
            serde_json::from_str::<JsonRpcRequest>(r#"{"jsonrpc":"2.0","id":1.5,"method":"x"}"#);
        assert!(request.is_err());
    }
}
