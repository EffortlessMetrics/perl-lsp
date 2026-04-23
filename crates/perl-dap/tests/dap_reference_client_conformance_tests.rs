//! Reference-client DAP conformance workflow tests.
//!
//! These tests emulate the command ordering and response expectations used by
//! reference clients (for example the `vscode-mock-debug` style launch flow):
//! initialize -> launch -> setBreakpoints -> configurationDone -> stopped ->
//! threads -> stackTrace -> scopes -> variables -> continue -> terminated.
//!
//! Focus: verify positive-path protocol compliance over a realistic DAP surface,
//! not just rejected/invalid input handling.

mod common;

use common::{DapWorkflowSession, perl_available, workflow_timeout};
use perl_dap::DapMessage;
use serde_json::Value;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture_path(name: &str) -> Result<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("fixture path is not valid UTF-8: {path:?}"))
}

fn assert_response_envelope(
    msg: &DapMessage,
    expected_command: &str,
    expected_request_seq: i64,
) -> Result<(), String> {
    match msg {
        DapMessage::Response { seq, request_seq, command, .. } => {
            assert!(*seq > 0, "response seq must be positive");
            assert_eq!(*request_seq, expected_request_seq, "request_seq must echo input seq");
            assert_eq!(command, expected_command, "response command must echo request command");
            Ok(())
        }
        other => Err(format!("expected Response for {expected_command}, got {other:?}")),
    }
}

#[test]
// AC:5, AC:13, AC:16
fn test_reference_client_launch_surface_conformance() -> TestResult {
    if !perl_available() {
        eprintln!("test_reference_client_launch_surface_conformance: skipping — perl not on PATH");
        return Ok(());
    }

    let script = fixture_path("hello.pl").map_err(|e| e.to_string())?;
    let mut session = DapWorkflowSession::new(workflow_timeout()).map_err(|e| e.to_string())?;

    session.launch(&script).map_err(|e| e.to_string())?;

    let breakpoints_body = session.set_breakpoints(&script, &[14]).map_err(|e| e.to_string())?;
    let breakpoints = breakpoints_body
        .as_ref()
        .and_then(|body| body.get("breakpoints"))
        .and_then(Value::as_array)
        .ok_or("setBreakpoints response must include breakpoints[]")?;
    assert_eq!(breakpoints.len(), 1, "expected one configured breakpoint");

    session.configuration_done().map_err(|e| e.to_string())?;

    let stopped = session.wait_stopped().map_err(|e| e.to_string())?;
    assert!(
        ["breakpoint", "entry", "step"].contains(&stopped.reason.as_str()),
        "unexpected stopped reason: {}",
        stopped.reason
    );

    let threads_msg = session.request("threads", None);
    assert_response_envelope(&threads_msg, "threads", 5).map_err(|e| e.to_string())?;
    let threads_body =
        session.expect_success(&threads_msg, "threads").map_err(|e| e.to_string())?;
    let threads = threads_body
        .as_ref()
        .and_then(|b| b.get("threads"))
        .and_then(Value::as_array)
        .ok_or("threads response must include threads[]")?;
    assert!(!threads.is_empty(), "threads[] must not be empty after launch stop");

    let (frame_id, source_path, frame_line) =
        session.stack_trace(stopped.thread_id).map_err(|e| e.to_string())?;
    assert!(frame_id > 0, "stackTrace top frame id must be > 0");
    assert!(!source_path.is_empty(), "stackTrace top frame should include source.path");
    assert!(frame_line > 0, "stackTrace top frame should include 1-based line");

    let globals_ref = session.scopes_globals_ref(frame_id).map_err(|e| e.to_string())?;
    assert!(globals_ref >= 0, "Globals scope variablesReference must be non-negative");

    let vars = session.variables(globals_ref).map_err(|e| e.to_string())?;
    assert!(
        vars.iter().all(|v| v.get("name").is_some() && v.get("variablesReference").is_some()),
        "each variable entry should include name + variablesReference"
    );

    session.continue_exec(stopped.thread_id).map_err(|e| e.to_string())?;
    let _ = session.drain_until_event("terminated");
    session.disconnect().map_err(|e| e.to_string())?;

    Ok(())
}
