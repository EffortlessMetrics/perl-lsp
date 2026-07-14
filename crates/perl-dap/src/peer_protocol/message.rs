//! The peer-protocol message envelope.
//!
//! Structurally identical to the DAP envelope family (a `type`-tagged
//! request/response/event), so the future ptkdb side can reuse ordinary
//! DAP-style message plumbing. Command and event names are strings (like DAP);
//! typed argument/body payloads live in [`super::payloads`].

use serde::{Deserialize, Serialize};

/// A request from one peer to the other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerRequest {
    /// Monotonic sequence number from the sender.
    pub seq: i64,
    /// Command name (see [`command`]).
    pub command: String,
    /// Command arguments, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// A response to a [`PeerRequest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerResponse {
    /// Monotonic sequence number from the responder.
    pub seq: i64,
    /// The `seq` of the request being answered.
    pub request_seq: i64,
    /// Whether the request succeeded.
    pub success: bool,
    /// The command being answered (echoed).
    pub command: String,
    /// Error message when `success` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response body, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// An asynchronous event from one peer to the other.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerEvent {
    /// Monotonic sequence number from the sender.
    pub seq: i64,
    /// Event name (see [`event`]).
    pub event: String,
    /// Event body, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

/// Any peer-protocol message.
///
/// Internally tagged on `type` (`"request"` / `"response"` / `"event"`), matching
/// the DAP envelope convention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PeerMessage {
    /// A request.
    Request(PeerRequest),
    /// A response.
    Response(PeerResponse),
    /// An event.
    Event(PeerEvent),
}

impl PeerMessage {
    /// The `seq` of the underlying message.
    #[must_use]
    pub fn seq(&self) -> i64 {
        match self {
            PeerMessage::Request(r) => r.seq,
            PeerMessage::Response(r) => r.seq,
            PeerMessage::Event(e) => e.seq,
        }
    }
}

/// Well-known command names (host↔peer requests).
pub mod command {
    /// Peer→host handshake.
    pub const HELLO: &str = "peer/hello";
    /// Either→either graceful shutdown.
    pub const GOODBYE: &str = "peer/goodbye";
    /// Host→peer: replace source breakpoints.
    pub const SET_BREAKPOINTS: &str = "debugger/setBreakpoints";
    /// Host→peer: replace function breakpoints.
    pub const SET_FUNCTION_BREAKPOINTS: &str = "debugger/setFunctionBreakpoints";
    /// Host→peer: resume.
    pub const CONTINUE: &str = "debugger/continue";
    /// Host→peer: step over.
    pub const NEXT: &str = "debugger/next";
    /// Host→peer: step in.
    pub const STEP_IN: &str = "debugger/stepIn";
    /// Host→peer: step out.
    pub const STEP_OUT: &str = "debugger/stepOut";
    /// Host→peer: pause.
    pub const PAUSE: &str = "debugger/pause";
    /// Host→peer: fetch stack trace.
    pub const STACK_TRACE: &str = "debugger/stackTrace";
    /// Host→peer: fetch scopes for a frame.
    pub const SCOPES: &str = "debugger/scopes";
    /// Host→peer: fetch variables for a reference.
    pub const VARIABLES: &str = "debugger/variables";
    /// Host→peer: evaluate an expression.
    pub const EVALUATE: &str = "debugger/evaluate";
    /// Host→peer: disconnect.
    pub const DISCONNECT: &str = "debugger/disconnect";
}

/// Well-known event names (peer→host, mostly).
pub mod event {
    /// Peer is initialized and ready for configuration.
    pub const INITIALIZED: &str = "debugger/initialized";
    /// The debuggee stopped.
    pub const STOPPED: &str = "debugger/stopped";
    /// The debuggee resumed.
    pub const CONTINUED: &str = "debugger/continued";
    /// The debuggee produced output.
    pub const OUTPUT: &str = "debugger/output";
    /// The session terminated.
    pub const TERMINATED: &str = "debugger/terminated";
    /// New static source facts are available.
    pub const SOURCE_FACTS: &str = "debugger/sourceFacts";
    /// Breakpoints changed state.
    pub const BREAKPOINTS_CHANGED: &str = "debugger/breakpointsChanged";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_message_serializes_with_type_tag() {
        let msg = PeerMessage::Request(PeerRequest {
            seq: 1,
            command: command::HELLO.to_string(),
            arguments: None,
        });
        let json = serde_json::to_value(&msg).expect("serialize");
        assert_eq!(json["type"], "request");
        assert_eq!(json["command"], "peer/hello");
        assert_eq!(json["seq"], 1);
    }

    #[test]
    fn response_uses_camel_case_request_seq() {
        let msg = PeerMessage::Response(PeerResponse {
            seq: 2,
            request_seq: 1,
            success: true,
            command: command::HELLO.to_string(),
            message: None,
            body: None,
        });
        let json = serde_json::to_value(&msg).expect("serialize");
        assert_eq!(json["type"], "response");
        assert_eq!(json["requestSeq"], 1);
        assert!(json.get("request_seq").is_none());
    }

    #[test]
    fn event_round_trips() {
        let msg = PeerMessage::Event(PeerEvent {
            seq: 10,
            event: event::STOPPED.to_string(),
            body: Some(serde_json::json!({"reason": "breakpoint"})),
        });
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: PeerMessage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(msg, back);
        assert_eq!(back.seq(), 10);
    }
}
