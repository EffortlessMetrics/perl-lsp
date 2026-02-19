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
    /// Supports function breakpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_function_breakpoints: Option<bool>,
    /// Supports setting variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_set_variable: Option<bool>,
    /// Supports value formatting options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_value_formatting_options: Option<bool>,
    /// Supports terminating the debuggee on disconnect
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_terminate_debuggee: Option<bool>,
    /// Supports stepping backwards
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_step_back: Option<bool>,
    /// Supports data breakpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_data_breakpoints: Option<bool>,
    /// Exception breakpoint filters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception_breakpoint_filters: Option<Vec<ExceptionBreakpointFilter>>,
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

// ============================================================================
// Stack/Variables/Scopes Request Arguments (AC8)
// ============================================================================

/// Arguments for stackTrace request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArguments {
    /// Thread to retrieve the stack trace for
    pub thread_id: i64,
    /// Index of the first frame to return (0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_frame: Option<i64>,
    /// Maximum number of frames to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<i64>,
}

/// Arguments for scopes request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArguments {
    /// Frame identifier to retrieve scopes for
    pub frame_id: i64,
}

/// Arguments for variables request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesArguments {
    /// Reference to the variable container
    pub variables_reference: i64,
    /// Optional filter ("indexed" or "named")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Start index of variables to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    /// Number of variables to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

/// Arguments for evaluate request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArguments {
    /// Expression to evaluate
    pub expression: String,
    /// Stack frame context for evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<i64>,
    /// Evaluation context ("watch", "repl", "hover", "clipboard")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Whether side effects are allowed during evaluation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_side_effects: Option<bool>,
}

// ============================================================================
// Control Flow Request Arguments (AC9)
// ============================================================================

/// Arguments for continue request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueArguments {
    /// Thread to continue
    pub thread_id: i64,
}

/// Arguments for next (step over) request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextArguments {
    /// Thread to step
    pub thread_id: i64,
}

/// Arguments for stepIn request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInArguments {
    /// Thread to step into
    pub thread_id: i64,
}

/// Arguments for stepOut request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutArguments {
    /// Thread to step out of
    pub thread_id: i64,
}

/// Arguments for pause request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseArguments {
    /// Thread to pause
    pub thread_id: i64,
}

// ============================================================================
// Variable Modification Request Arguments (AC8)
// ============================================================================

/// Arguments for setVariable request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVariableArguments {
    /// Reference to the variable container
    pub variables_reference: i64,
    /// Name of the variable to set
    pub name: String,
    /// New value for the variable
    pub value: String,
}

// ============================================================================
// Session Lifecycle Request Arguments (AC5)
// ============================================================================

/// Arguments for disconnect request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectArguments {
    /// Whether to restart the debug session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<bool>,
    /// Whether to terminate the debuggee process
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminate_debuggee: Option<bool>,
}

/// Arguments for terminate request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminateArguments {
    /// Whether to restart after termination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<bool>,
}

// ============================================================================
// Extended Breakpoint Request Arguments (AC7)
// ============================================================================

/// Arguments for setFunctionBreakpoints request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFunctionBreakpointsArguments {
    /// Function breakpoints to set
    pub breakpoints: Vec<FunctionBreakpoint>,
}

/// Arguments for setExceptionBreakpoints request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetExceptionBreakpointsArguments {
    /// Exception filter IDs to activate
    pub filters: Vec<String>,
    /// Additional exception filter options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_options: Option<Vec<ExceptionFilterOption>>,
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Function breakpoint specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionBreakpoint {
    /// Name of the function to break on
    pub name: String,
    /// Breakpoint condition expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit-condition expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
}

/// Exception filter option for fine-grained exception breakpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionFilterOption {
    /// ID of the exception filter
    pub filter_id: String,
    /// Condition expression for the filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// Exception breakpoint filter descriptor (reported in capabilities)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointFilter {
    /// Unique filter identifier
    pub filter: String,
    /// Human-readable label for the filter
    pub label: String,
    /// Whether this filter is enabled by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// A thread in the debuggee
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    /// Thread identifier
    pub id: i64,
    /// Human-readable thread name
    pub name: String,
}

/// A scope within a stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Scope name (e.g., "Locals", "Globals")
    pub name: String,
    /// Presentation hint ("arguments", "locals", "registers")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<String>,
    /// Reference to the variables in this scope
    pub variables_reference: i64,
    /// Whether fetching variables is expensive
    pub expensive: bool,
}

/// A variable in the debuggee
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVariable {
    /// Variable name
    pub name: String,
    /// String representation of the variable value
    pub value: String,
    /// Type of the variable
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Reference for child variables (0 means no children)
    pub variables_reference: i64,
    /// Number of named child variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i64>,
    /// Number of indexed child variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<i64>,
}

/// A stack frame in the call stack
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolStackFrame {
    /// Frame identifier
    pub id: i64,
    /// Name of the frame (typically the function name)
    pub name: String,
    /// Source location of the frame
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line number (1-based)
    pub line: i64,
    /// Column number (1-based)
    pub column: i64,
    /// End line number (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,
    /// End column number (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i64>,
}

// ============================================================================
// Response Body Types
// ============================================================================

/// Response body for threads request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadsResponseBody {
    /// All threads in the debuggee
    pub threads: Vec<Thread>,
}

/// Response body for stackTrace request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceResponseBody {
    /// Stack frames in the call stack
    pub stack_frames: Vec<ProtocolStackFrame>,
    /// Total number of frames available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_frames: Option<i64>,
}

/// Response body for scopes request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesResponseBody {
    /// Scopes in the specified stack frame
    pub scopes: Vec<Scope>,
}

/// Response body for variables request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesResponseBody {
    /// Variables in the specified scope or container
    pub variables: Vec<ProtocolVariable>,
}

/// Response body for continue request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueResponseBody {
    /// Whether all threads were continued
    pub all_threads_continued: bool,
}

/// Response body for evaluate request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponseBody {
    /// String representation of the evaluation result
    pub result: String,
    /// Type of the result
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Reference for structured result (0 means no children)
    pub variables_reference: i64,
}

/// Response body for setVariable request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVariableResponseBody {
    /// New string representation of the variable value
    pub value: String,
    /// Type of the variable after setting
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Reference for child variables (0 means no children)
    pub variables_reference: i64,
}
