//! BDD-style workflow coverage for perl-dap.
//!
//! These scenarios use Given/When/Then structure to validate
//! user-visible adapter behaviors through DAP requests.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::fs::write;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;
use tempfile::tempdir;

struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, msg: &str) {
        eprintln!("[{}] Given {msg}", self.name);
    }

    fn when(&self, msg: &str) {
        eprintln!("[{}] When {msg}", self.name);
    }

    fn then(&self, msg: &str) {
        eprintln!("[{}] Then {msg}", self.name);
    }
}

#[allow(clippy::panic)]
fn expect_response(msg: DapMessage, command: &str, expected_success: bool) -> Option<Value> {
    match msg {
        DapMessage::Response { command: actual, success, body, .. } => {
            assert_eq!(actual, command, "unexpected command");
            assert_eq!(success, expected_success, "unexpected success value");
            body
        }
        _ => panic!("expected response for command {command}"),
    }
}

fn expect_event(rx: &Receiver<DapMessage>, event_name: &str) {
    let msg = perl_tdd_support::must(rx.recv_timeout(Duration::from_millis(500)));
    match msg {
        DapMessage::Event { event, .. } => assert_eq!(event, event_name),
        _ => panic!("expected event {event_name}"),
    }
}

#[test]
fn bdd_initialize_emits_initialized_event_and_capabilities() {
    let scenario = BddScenario::new("initialize handshake advertises capabilities");
    scenario.given("a new debug adapter with an event channel");

    let (tx, rx) = channel();
    let mut adapter = DebugAdapter::new();
    adapter.set_event_sender(tx);

    scenario.when("the client sends initialize");
    let response = adapter.handle_request(1, "initialize", None);

    scenario.then("the adapter returns capability flags and emits initialized event");
    let body = expect_response(response, "initialize", true)
        .unwrap_or_else(|| panic!("initialize should return capabilities"));
    assert_eq!(body.get("supportsConfigurationDoneRequest").and_then(Value::as_bool), Some(true));
    assert!(body.get("supportsStepInTargetsRequest").and_then(Value::as_bool).is_some());
    expect_event(&rx, "initialized");
}

#[test]
fn bdd_launch_without_configuration_returns_actionable_error() {
    let scenario = BddScenario::new("launch request requires launch configuration");
    scenario.given("an adapter that has completed initialize");

    let mut adapter = DebugAdapter::new();
    let init = adapter.handle_request(1, "initialize", None);
    let _ = expect_response(init, "initialize", true);

    scenario.when("the client sends launch without arguments");
    let launch = adapter.handle_request(2, "launch", None);

    scenario.then("the adapter returns a failed response with guidance");
    match launch {
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "launch");
            assert!(!success, "launch should fail without config");
            let message = message.unwrap_or_default();
            assert!(message.contains("launch configuration"));
            assert!(message.contains("program"));
        }
        _ => panic!("expected launch response"),
    }
}

#[test]
fn bdd_set_breakpoints_marks_comment_line_unverified() {
    let scenario = BddScenario::new("breakpoint validation distinguishes code from comments");
    scenario.given("a Perl source file with a comment line and executable line");

    let dir = perl_tdd_support::must(tempdir());
    let script_path = dir.path().join("breakpoints.pl");
    perl_tdd_support::must(write(&script_path, "# comment only\nmy $x = 1;\nprint $x;\n"));

    let mut adapter = DebugAdapter::new();

    scenario.when("the client sets breakpoints on both lines");
    let response = adapter.handle_request(
        3,
        "setBreakpoints",
        Some(json!({
            "source": { "path": script_path.to_string_lossy() },
            "breakpoints": [{ "line": 1 }, { "line": 2 }]
        })),
    );

    scenario.then("the comment line is unverified while executable code is verified");
    let body = expect_response(response, "setBreakpoints", true)
        .unwrap_or_else(|| panic!("setBreakpoints should return a body"));
    let breakpoints = body
        .get("breakpoints")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("missing breakpoints array"));
    assert_eq!(breakpoints.len(), 2);

    let first_verified = breakpoints
        .first()
        .and_then(|bp| bp.get("verified"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let second_verified = breakpoints
        .get(1)
        .and_then(|bp| bp.get("verified"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    assert!(!first_verified, "comment line must be unverified");
    assert!(second_verified, "code line must be verified");
}
