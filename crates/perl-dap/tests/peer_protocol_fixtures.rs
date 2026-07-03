//! Drift guard: the golden fixtures in `fixtures/debug-peer/` must deserialize
//! into the current `perl_dap::peer_protocol` wire types. If a wire type changes
//! incompatibly, these fail.

use std::path::PathBuf;

use perl_dap::peer_protocol::message::{PeerMessage, command, event};
use perl_dap::peer_protocol::payloads::{
    HelloArgs, HelloResponseBody, SetBreakpointsArgs, SetBreakpointsResponseBody, StoppedEventBody,
};

fn fixture(name: &str) -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/debug-peer").join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

fn parse(name: &str) -> PeerMessage {
    serde_json::from_str(&fixture(name)).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

#[test]
fn hello_request_fixture_matches_wire_types() {
    let msg = parse("hello_request.json");
    let PeerMessage::Request(req) = msg else {
        panic!("expected request");
    };
    assert_eq!(req.command, command::HELLO);
    let args: HelloArgs = serde_json::from_value(req.arguments.expect("args")).expect("hello args");
    assert_eq!(args.peer, "Devel::ptkdb");
    assert_eq!(args.protocol_version, perl_dap::peer_protocol::PROTOCOL_VERSION);
    assert!(args.capabilities.can_set_breakpoints);
    assert!(args.capabilities.can_condition_breakpoints);
}

#[test]
fn hello_response_fixture_matches_wire_types() {
    let PeerMessage::Response(resp) = parse("hello_response.json") else {
        panic!("expected response");
    };
    assert!(resp.success);
    assert_eq!(resp.command, command::HELLO);
    let body: HelloResponseBody =
        serde_json::from_value(resp.body.expect("body")).expect("hello response body");
    assert_eq!(body.protocol_version, perl_dap::peer_protocol::PROTOCOL_VERSION);
    assert!(body.capabilities.wants_source_facts);
}

#[test]
fn stopped_event_fixture_matches_wire_types() {
    let PeerMessage::Event(ev) = parse("stopped_event.json") else {
        panic!("expected event");
    };
    assert_eq!(ev.event, event::STOPPED);
    let body: StoppedEventBody =
        serde_json::from_value(ev.body.expect("body")).expect("stopped body");
    assert_eq!(body.reason, "breakpoint");
    assert_eq!(body.line, Some(42));
}

#[test]
fn set_breakpoints_request_fixture_matches_wire_types() {
    let PeerMessage::Request(req) = parse("set_breakpoints_request.json") else {
        panic!("expected request");
    };
    assert_eq!(req.command, command::SET_BREAKPOINTS);
    let args: SetBreakpointsArgs =
        serde_json::from_value(req.arguments.expect("args")).expect("set bp args");
    assert_eq!(args.breakpoints.len(), 2);
    assert_eq!(args.breakpoints[0].condition.as_deref(), Some("$x > 10"));
}

#[test]
fn set_breakpoints_response_fixture_preserves_order() {
    let PeerMessage::Response(resp) = parse("set_breakpoints_response.json") else {
        panic!("expected response");
    };
    let body: SetBreakpointsResponseBody =
        serde_json::from_value(resp.body.expect("body")).expect("set bp resp");
    assert_eq!(body.breakpoints.len(), 2);
    assert_eq!(body.breakpoints[0].id, 1);
    assert!(body.breakpoints[0].verified);
    assert!(!body.breakpoints[1].verified);
}
