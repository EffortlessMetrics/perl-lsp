//! BDD-style workflow coverage for perl-dap request/response behavior.
//!
//! These scenarios use Given/When/Then logging to describe the user-facing
//! debugging workflow while asserting concrete protocol outcomes.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::sync::mpsc::{Receiver, channel};

struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

fn assert_success_response(message: DapMessage, command: &str) -> Option<Value> {
    match message {
        DapMessage::Response { success, command: actual, body, message, .. } => {
            assert_eq!(actual, command, "response command mismatch");
            assert!(
                success,
                "expected `{command}` request to succeed but failed: {}",
                message.unwrap_or_else(|| "<no message>".to_string())
            );
            body
        }
        other => panic!("expected response for `{command}`, got: {other:?}"),
    }
}

fn recv_initialized_event(rx: &Receiver<DapMessage>) -> DapMessage {
    let event = rx.recv().unwrap_or_else(|error| panic!("failed to receive event: {error}"));
    match &event {
        DapMessage::Event { event, .. } => assert_eq!(event, "initialized"),
        other => panic!("expected initialized event, got: {other:?}"),
    }
    event
}

#[test]
fn bdd_initialize_advertises_capabilities_and_emits_initialized_event() {
    let scenario = BddScenario::new(
        "Initialize handshake advertises capabilities and emits initialized event",
    );
    scenario.given("a new debug adapter and an event channel");

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    scenario.when("the client sends initialize");
    let body = assert_success_response(
        adapter.handle_request(
            1,
            "initialize",
            Some(json!({
                "clientID": "vscode",
                "adapterID": "perl-dap",
                "linesStartAt1": true,
                "columnsStartAt1": true
            })),
        ),
        "initialize",
    )
    .unwrap_or(Value::Null);

    scenario.then("the adapter reports core capabilities and emits initialized");
    assert_eq!(body.get("supportsConfigurationDoneRequest").and_then(Value::as_bool), Some(true));
    assert_eq!(body.get("supportsEvaluateForHovers").and_then(Value::as_bool), Some(true));
    assert_eq!(body.get("supportsSetVariable").and_then(Value::as_bool), Some(true));

    let _initialized = recv_initialized_event(&rx);
}

#[test]
fn bdd_configuration_done_is_tolerated_before_initialize() {
    let scenario = BddScenario::new("Configuration done is tolerated before initialize");
    scenario.given("a fresh debug adapter with no initialize request");

    let mut adapter = DebugAdapter::new();

    scenario.when("the client sends configurationDone immediately");
    let response = adapter.handle_request(1, "configurationDone", None);

    scenario.then("the adapter tolerates the call and responds successfully");
    let body = assert_success_response(response, "configurationDone");
    assert!(body.is_none() || body == Some(Value::Null));
}

#[test]
fn bdd_disconnect_succeeds_without_active_session() {
    let scenario = BddScenario::new("Disconnect succeeds even when no debug session is active");
    scenario.given("an initialized adapter that has not launched or attached a program");

    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let _ = assert_success_response(adapter.handle_request(1, "initialize", None), "initialize");

    scenario.when("the client sends disconnect");
    let response_body = assert_success_response(
        adapter.handle_request(2, "disconnect", Some(json!({ "terminateDebuggee": false }))),
        "disconnect",
    );

    scenario.then("the adapter returns a successful disconnect response body");
    assert!(response_body.is_none() || response_body == Some(Value::Null));
}
