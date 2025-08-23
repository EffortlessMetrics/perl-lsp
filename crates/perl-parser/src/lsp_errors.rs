//! LSP Error helpers
//!
//! Provides consistent error responses for the LSP server

use crate::lsp_server::JsonRpcError;
use serde_json::{Value, json};

/// LSP error codes (from the spec)
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_START: i32 = -32099;
    pub const SERVER_ERROR_END: i32 = -32000;
    pub const SERVER_NOT_INITIALIZED: i32 = -32002;
    pub const UNKNOWN_ERROR_CODE: i32 = -32001;
    pub const REQUEST_CANCELLED: i32 = -32800;
    pub const CONTENT_MODIFIED: i32 = -32801;
    pub const REQUEST_FAILED: i32 = -32803;
}

/// Create a method not found error
pub fn method_not_found(method: &str) -> JsonRpcError {
    JsonRpcError {
        code: error_codes::METHOD_NOT_FOUND,
        message: format!("Method '{}' not found or not supported", method),
        data: None,
    }
}

/// Create a method not found error for unadvertised features
pub fn method_not_advertised() -> JsonRpcError {
    JsonRpcError {
        code: error_codes::METHOD_NOT_FOUND,
        message: "Method not advertised in server capabilities".to_string(),
        data: None,
    }
}

/// Create a server cancelled error
pub fn server_cancelled(message: &str) -> Value {
    json!({
        "code": error_codes::REQUEST_CANCELLED,
        "message": message
    })
}

/// Create an invalid params error
pub fn invalid_params(message: &str) -> Value {
    json!({
        "code": error_codes::INVALID_PARAMS,
        "message": message
    })
}

/// Create an internal error
pub fn internal_error(message: &str) -> Value {
    json!({
        "code": error_codes::INTERNAL_ERROR,
        "message": message
    })
}

/// Check if a feature is advertised
pub struct AdvertisedFeatures {
    pub code_lens: bool,
    pub call_hierarchy: bool,
    pub type_hierarchy: bool,
    pub inlay_hints: bool,
    pub semantic_tokens: bool,
    pub code_actions: bool,
    pub rename: bool,
    pub document_links: bool,
    pub selection_ranges: bool,
    pub on_type_formatting: bool,
    pub pull_diagnostics: bool,
}

impl Default for AdvertisedFeatures {
    fn default() -> Self {
        // Match production BuildFlags
        Self {
            code_lens: false,
            call_hierarchy: false,
            type_hierarchy: false,
            inlay_hints: true,
            semantic_tokens: true,
            code_actions: true,
            rename: true,
            document_links: true,
            selection_ranges: true,
            on_type_formatting: true,
            pull_diagnostics: true,
        }
    }
}

impl AdvertisedFeatures {
    /// Create GA-lock features (conservative set)
    pub fn ga_lock() -> Self {
        Self {
            code_lens: false,
            call_hierarchy: false,
            type_hierarchy: false,
            inlay_hints: false,
            semantic_tokens: false,
            code_actions: false,
            rename: false,
            document_links: false,
            selection_ranges: false,
            on_type_formatting: false,
            pull_diagnostics: false,
        }
    }

    /// Check if a method should be refused
    pub fn should_refuse(&self, method: &str) -> bool {
        match method {
            "textDocument/codeLens" => !self.code_lens,
            "textDocument/prepareCallHierarchy"
            | "callHierarchy/incomingCalls"
            | "callHierarchy/outgoingCalls" => !self.call_hierarchy,
            "textDocument/prepareTypeHierarchy"
            | "typeHierarchy/supertypes"
            | "typeHierarchy/subtypes" => !self.type_hierarchy,
            _ => false, // Allow by default for core features
        }
    }
}
