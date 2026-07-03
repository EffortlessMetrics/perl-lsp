//! Typed request-argument, response-body, and event-body payloads for the peer
//! protocol, plus the `camelCase` wire types they use.
//!
//! These are the **wire contract**. They are intentionally separate from
//! [`crate::model`] so the internal model can evolve without breaking peers.
//! The external peer backend translates between these and the model.

use serde::{Deserialize, Serialize};

use super::capabilities::{HostReportedCapabilities, PeerReportedCapabilities};

// ---------------------------------------------------------------------------
// Wire value types
// ---------------------------------------------------------------------------

/// A source on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSource {
    /// Absolute path.
    pub path: String,
    /// Optional short name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional reference for path-less sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,
}

/// A requested source breakpoint on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSourceBreakpoint {
    /// 1-based line.
    pub line: u32,
    /// Optional 1-based column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Optional condition expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Optional hit-count condition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    /// Optional logpoint message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
}

/// A resolved breakpoint on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireResolvedBreakpoint {
    /// Backend-assigned id.
    pub id: i64,
    /// Whether the engine bound it.
    pub verified: bool,
    /// Line it actually landed on.
    pub line: u32,
    /// Optional column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Optional human-readable note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// A stack frame on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireStackFrame {
    /// Frame id.
    pub id: i64,
    /// Frame name.
    pub name: String,
    /// Source.
    pub source: WireSource,
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
}

/// A scope on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireScope {
    /// Scope name.
    pub name: String,
    /// Handle to fetch the scope's variables.
    pub variables_reference: i64,
    /// Whether expansion is expensive.
    #[serde(default)]
    pub expensive: bool,
}

/// A variable on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireVariable {
    /// Variable name.
    pub name: String,
    /// Rendered value.
    pub value: String,
    /// Type/ref-kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Handle to expand children (0 = none).
    #[serde(default)]
    pub variables_reference: i64,
    /// Number of indexed children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<u64>,
    /// Number of named children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<u64>,
}

/// A subroutine on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireSubroutine {
    /// Fully-qualified sub name.
    pub name: String,
    /// Source it is defined in.
    pub source: WireSource,
    /// 1-based first line.
    pub start_line: u32,
    /// 1-based last line.
    pub end_line: u32,
}

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

/// `peer/hello` request arguments (peer→host).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloArgs {
    /// Peer name, e.g. `"Devel::ptkdb"`.
    pub peer: String,
    /// Peer version string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_version: Option<String>,
    /// Protocol version the peer speaks.
    pub protocol_version: String,
    /// Peer capabilities.
    #[serde(default)]
    pub capabilities: PeerReportedCapabilities,
}

/// `peer/hello` response body (host→peer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloResponseBody {
    /// Protocol version the host accepts.
    pub protocol_version: String,
    /// Session identifier.
    pub session_id: String,
    /// What the host wants from the peer.
    pub capabilities: HostReportedCapabilities,
}

// ---------------------------------------------------------------------------
// Requests (host→peer)
// ---------------------------------------------------------------------------

/// `debugger/setBreakpoints` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArgs {
    /// Source the breakpoints apply to.
    pub source: WireSource,
    /// Full replacement breakpoint set.
    pub breakpoints: Vec<WireSourceBreakpoint>,
}

/// `debugger/setBreakpoints` (and `setFunctionBreakpoints`) response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsResponseBody {
    /// Resolved breakpoints in the same order as the request.
    pub breakpoints: Vec<WireResolvedBreakpoint>,
}

/// `debugger/setFunctionBreakpoints` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFunctionBreakpointsArgs {
    /// Fully-qualified sub names.
    pub names: Vec<String>,
}

/// A `threadId`-only argument (continue/next/stepIn/stepOut/pause).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadArgs {
    /// Thread to act on.
    pub thread_id: i64,
}

/// `debugger/stackTrace` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArgs {
    /// Thread to inspect.
    pub thread_id: i64,
    /// First frame index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_frame: Option<u32>,
    /// Max frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levels: Option<u32>,
}

/// `debugger/stackTrace` response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceResponseBody {
    /// The frames.
    pub stack_frames: Vec<WireStackFrame>,
}

/// `debugger/scopes` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArgs {
    /// Frame to inspect.
    pub frame_id: i64,
}

/// `debugger/scopes` response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesResponseBody {
    /// The scopes.
    pub scopes: Vec<WireScope>,
}

/// `debugger/variables` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesArgs {
    /// Reference to expand.
    pub variables_reference: i64,
}

/// `debugger/variables` response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesResponseBody {
    /// The variables.
    pub variables: Vec<WireVariable>,
}

/// `debugger/evaluate` arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArgs {
    /// Expression to evaluate.
    pub expression: String,
    /// Frame context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<i64>,
    /// Evaluate context (`watch`/`repl`/`hover`/`variables`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// `debugger/evaluate` response body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponseBody {
    /// Rendered result.
    pub result: String,
    /// Type/ref-kind, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Handle to expand children (0 = none).
    #[serde(default)]
    pub variables_reference: i64,
}

// ---------------------------------------------------------------------------
// Events (peer→host)
// ---------------------------------------------------------------------------

/// `debugger/stopped` event body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEventBody {
    /// Why it stopped (`breakpoint`/`step`/`entry`/`exception`/`pause`/…).
    pub reason: String,
    /// Which thread stopped.
    pub thread_id: i64,
    /// Where it stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<WireSource>,
    /// 1-based line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// 1-based column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

/// `debugger/continued` event body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuedEventBody {
    /// Which thread resumed.
    pub thread_id: i64,
}

/// `debugger/output` event body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEventBody {
    /// `stdout`/`stderr`/`console`.
    pub category: String,
    /// The output text.
    pub output: String,
}

/// `debugger/terminated` event body.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminatedEventBody {
    /// Exit code, if the debuggee exited normally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
}

/// `debugger/sourceFacts` event body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFactsEventBody {
    /// The source the facts describe.
    pub source: WireSource,
    /// Breakable line candidates.
    #[serde(default)]
    pub breakable_lines: Vec<u32>,
    /// Subroutines defined in the source.
    #[serde(default)]
    pub subroutines: Vec<WireSubroutine>,
}

/// `debugger/breakpointsChanged` event body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointsChangedEventBody {
    /// The updated breakpoints.
    pub breakpoints: Vec<WireResolvedBreakpoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_args_round_trip_camel_case() {
        let args = HelloArgs {
            peer: "Devel::ptkdb".to_string(),
            peer_version: Some("1.1091".to_string()),
            protocol_version: super::super::PROTOCOL_VERSION.to_string(),
            capabilities: PeerReportedCapabilities {
                can_continue: true,
                can_step: true,
                can_evaluate: true,
                can_set_breakpoints: true,
                ..Default::default()
            },
        };
        let json = serde_json::to_value(&args).expect("serialize");
        assert_eq!(json["peerVersion"], "1.1091");
        assert_eq!(json["protocolVersion"], super::super::PROTOCOL_VERSION);
        assert_eq!(json["capabilities"]["canSetBreakpoints"], true);
        let back: HelloArgs = serde_json::from_value(json).expect("deserialize");
        assert_eq!(args, back);
    }

    #[test]
    fn stopped_event_body_parses_ptkdb_shape() {
        let body: StoppedEventBody = serde_json::from_str(
            r#"{"reason":"breakpoint","threadId":1,"source":{"path":"/work/script.pl"},"line":42,"column":1}"#,
        )
        .expect("deserialize");
        assert_eq!(body.reason, "breakpoint");
        assert_eq!(body.thread_id, 1);
        assert_eq!(body.line, Some(42));
        assert_eq!(body.source.expect("source").path, "/work/script.pl");
    }

    #[test]
    fn set_breakpoints_response_preserves_order() {
        let body = SetBreakpointsResponseBody {
            breakpoints: vec![
                WireResolvedBreakpoint {
                    id: 1,
                    verified: true,
                    line: 10,
                    column: None,
                    message: None,
                },
                WireResolvedBreakpoint {
                    id: 2,
                    verified: false,
                    line: 20,
                    column: None,
                    message: Some("no code on line".to_string()),
                },
            ],
        };
        let json = serde_json::to_string(&body).expect("serialize");
        let back: SetBreakpointsResponseBody = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.breakpoints[0].id, 1);
        assert_eq!(back.breakpoints[1].id, 2);
        assert!(!back.breakpoints[1].verified);
    }
}
