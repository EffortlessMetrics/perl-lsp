//! DAP Protocol Types
//!
//! This module defines the JSON-RPC 2.0 message types for the Debug Adapter Protocol.
//! It follows the DAP 1.x specification with support for Perl-specific features.
//!
//! # Message Transport
//!
//! Messages are framed using Content-Length headers:
//! ```text
//! Content-Length: <length>\r\n
//! \r\n
//! <JSON message>
//! ```
//!
//! # References
//!
//! - [DAP Protocol Schema](../../docs/DAP_PROTOCOL_SCHEMA.md)
//! - [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base request message
///
/// All DAP requests follow this structure with command-specific arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// Sequence number (incremented for each message)
    pub seq: i64,
    /// Message type (always "request")
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Command name (e.g., "initialize", "setBreakpoints")
    pub command: String,
    /// Command-specific arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Base response message
///
/// All DAP responses follow this structure with command-specific body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    /// Sequence number
    pub seq: i64,
    /// Message type (always "response")
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Sequence number of corresponding request
    pub request_seq: i64,
    /// Whether the request succeeded
    pub success: bool,
    /// Command name
    pub command: String,
    /// Error message (if success=false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Command-specific response body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Base event message
///
/// DAP events notify the client of state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Sequence number
    pub seq: i64,
    /// Message type (always "event")
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Event name (e.g., "initialized", "stopped")
    pub event: String,
    /// Event-specific body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

// ============================================================================
// Breakpoint Request/Response Types (AC7)
// ============================================================================

/// Source reference in breakpoint requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Absolute file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// File name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Source breakpoint in request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    /// Line number (1-based)
    pub line: i64,
    /// Column number (0-based, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// Breakpoint condition (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit-condition expression (optional), e.g. `>= 10` or `%2`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    /// Logpoint message (optional). When present, breakpoint logs and continues.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
}

/// Arguments for setBreakpoints request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArguments {
    /// Source file reference
    pub source: Source,
    /// Array of breakpoints to set (REPLACE semantics: clears existing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
    /// Whether source file was modified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_modified: Option<bool>,
}

/// Verified breakpoint in response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakpoint {
    /// Unique breakpoint identifier
    pub id: i64,
    /// Whether breakpoint was successfully verified
    pub verified: bool,
    /// Actual line number (may differ from requested if adjusted)
    pub line: i64,
    /// Actual column number (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// Error/warning message if not verified or adjusted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Response body for setBreakpoints request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsResponseBody {
    /// Array of verified breakpoints (SAME ORDER as request)
    pub breakpoints: Vec<Breakpoint>,
}

// ============================================================================
// Inline Values Request/Response Types (Custom)
// ============================================================================

/// Arguments for inlineValues request
///
/// This request is a lightweight, Perl-specific extension that mirrors the
/// LSP inlineValue provider using a source file and line range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineValuesArguments {
    /// Source file reference
    pub source: Source,
    /// Start line (1-based, inclusive)
    pub start_line: i64,
    /// End line (1-based, inclusive)
    pub end_line: i64,
}

/// Inline value hint for a single variable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineValueText {
    /// Line number (1-based)
    pub line: i64,
    /// Column number (1-based)
    pub column: i64,
    /// Rendered inline value text
    pub text: String,
}

/// Response body for inlineValues request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineValuesResponseBody {
    /// Inline values for the requested range
    pub inline_values: Vec<InlineValueText>,
}

// ============================================================================
// Initialize Request/Response Types (AC5)
// ============================================================================

/// Arguments for initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArguments {
    /// Client ID (e.g., "vscode")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Client name (e.g., "Visual Studio Code")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    /// Adapter ID (e.g., "perl-rs")
    pub adapter_id: String,
    /// Locale (e.g., "en-US")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Lines are 1-based
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines_start_at1: Option<bool>,
    /// Columns are 1-based
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns_start_at1: Option<bool>,
    /// Path format ("path" or "uri")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_format: Option<String>,
}

/// Response body for initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// Supports configuration done request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_configuration_done_request: Option<bool>,
    /// Supports evaluate for hovers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_evaluate_for_hovers: Option<bool>,
    /// Supports conditional breakpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_conditional_breakpoints: Option<bool>,
    /// Supports hit-conditional breakpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_hit_conditional_breakpoints: Option<bool>,
    /// Supports logpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_log_points: Option<bool>,
    /// Supports exception breakpoint options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_options: Option<bool>,
    /// Supports exception filter options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_exception_filter_options: Option<bool>,
    /// Supports terminate request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_terminate_request: Option<bool>,
    /// Supports custom inlineValues request for inline debug hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_inline_values: Option<bool>,
}

// ============================================================================
// Launch Request/Response Types
// ============================================================================

/// Arguments for launch request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequestArguments {
    /// Absolute path to Perl script
    pub program: String,
    /// Command-line arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Path to Perl executable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perl_path: Option<String>,
    /// Stop on entry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_on_entry: Option<bool>,
}

// ============================================================================
// Attach Request/Response Types
// ============================================================================

/// Arguments for attach request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachRequestArguments {
    /// Process ID to attach to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
    /// Host to connect to (for TCP attachment)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Port number for TCP attachment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Connection timeout in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
}
