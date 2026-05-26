//! Regression net for issue #353: removing `DapDispatcher`.
//!
//! `DapDispatcher` is deprecated since 0.2.0 (workspace is now 0.15.0) and
//! has no callers outside its own unit tests. Before its removal, these
//! tests pin the supported `DebugAdapter::handle_request` analogues of its
//! behaviors - the production code path used by `DapServer::run`.
//!
//! The tests below map to `DapDispatcher` tests in
//! `crates/perl-dap/src/dispatcher/mod.rs` (`mod tests`) where the behavior
//! is observable through the production request surface. Internal store state
//! coverage lives beside `DebugAdapter`, where private adapter state can be
//! inspected without adding public test-only API.
//!
//! Behavior intentionally **not** reproduced (documented as a deliberate
//! divergence in issue #353):
//!
//! - `DapDispatcher` rejected `configurationDone` before `initialize` with
//!   an error containing `"before initialized"`. `DebugAdapter` is more
//!   permissive and returns success regardless of initialization state.
//!   Since the production server already uses `DebugAdapter`, this is the
//!   behavior users have observed for many minor releases; the removal
//!   only deletes the unused strict check. The test
//!   `configuration_done_before_initialize_is_permissive` codifies the
//!   current behavior so it is not silently changed again.
//! - `DapDispatcher` had a failed-initialize no-event test. `DebugAdapter`
//!   accepts initialize arguments permissively and has no corresponding
//!   initialize failure path.

use perl_dap::{DapMessage, DebugAdapter};
use perl_tdd_support::{must, must_some};
use serde_json::json;
use std::io::Write;
use std::sync::mpsc::channel;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Mirror of `DapDispatcher::tests::create_test_perl_file`: produce a temp
/// `.pl` file with a known mix of executable lines, blank lines, and
/// comments so AST validation has something deterministic to verify.
fn create_test_perl_file() -> (NamedTempFile, String) {
    let mut file = must(NamedTempFile::with_suffix(".pl"));
    let perl_code = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
my $y = 2;
my $z = $x + $y;

if ($x > 0) {
    print "positive\n";
}

my @arr = (1, 2, 3);
while (my $item = shift @arr) {
    my $doubled = $item * 2;
    print "$doubled\n";
}

sub process {
    my ($value) = @_;
    my $result = $value * 2;
    return $result;
}

print "done\n";
my $final = process($x);
print "result: $final\n";
"#;
    must(file.write_all(perl_code.as_bytes()));
    must(file.flush());
    let path = file.path().to_string_lossy().to_string();
    (file, path)
}

// --- initialize ---------------------------------------------------------------

/// Mirrors `dispatcher::tests::test_handle_initialize`: `initialize` must
/// report the two capabilities its unit test asserts on, via the
/// production handler.
#[test]
fn initialize_reports_configuration_done_and_evaluate_for_hovers() {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(
        1,
        "initialize",
        Some(json!({
            "clientID": "vscode",
            "clientName": "Visual Studio Code",
            "adapterID": "perl-rs",
            "linesStartAt1": true,
            "columnsStartAt1": true,
        })),
    );

    let body = match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "initialize should succeed");
            assert_eq!(command, "initialize");
            must_some(body)
        }
        other => must(Err::<serde_json::Value, _>(format!("expected Response, got {other:?}"))),
    };

    let configuration_done =
        body.get("supportsConfigurationDoneRequest").and_then(|v| v.as_bool()).unwrap_or(false);
    let evaluate_for_hovers =
        body.get("supportsEvaluateForHovers").and_then(|v| v.as_bool()).unwrap_or(false);

    assert!(configuration_done, "supportsConfigurationDoneRequest must be true");
    assert!(evaluate_for_hovers, "supportsEvaluateForHovers must be true");
}

/// Mirrors `dispatcher::tests::test_initialize_emits_initialized_event`: a
/// successful initialize must emit a single `initialized` event with no
/// body.
#[test]
fn successful_initialize_emits_initialized_event_with_no_body() {
    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let response = adapter.handle_request(1, "initialize", None);
    assert!(
        matches!(response, DapMessage::Response { success: true, .. }),
        "initialize should succeed"
    );

    let event = must(rx.recv_timeout(Duration::from_millis(500)));
    match event {
        DapMessage::Event { event, body, .. } => {
            assert_eq!(event, "initialized");
            assert!(body.is_none(), "initialized event carries no body");
        }
        other => must(Err::<(), _>(format!("expected initialized Event, got {other:?}"))),
    }
}

// --- setBreakpoints -----------------------------------------------------------

/// Mirrors `dispatcher::tests::test_handle_set_breakpoints`: on a real
/// Perl file, AST-valid lines must come back `verified: true`.
///
/// Existing DebugAdapter coverage (e.g. `dap_comprehensive_test.rs`) only
/// exercised the unverified path (non-existent source path). This pins the
/// verified path through the same dispatch surface.
#[test]
fn set_breakpoints_marks_executable_lines_verified() {
    let (_keep, source_path) = create_test_perl_file();
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(
        1,
        "setBreakpoints",
        Some(json!({
            "source": { "path": source_path, "name": "script.pl" },
            "breakpoints": [
                { "line": 10 },
                { "line": 25 },
            ],
        })),
    );

    let body = match response {
        DapMessage::Response { success, command, body, .. } => {
            assert!(success, "setBreakpoints should succeed");
            assert_eq!(command, "setBreakpoints");
            must_some(body)
        }
        other => must(Err::<serde_json::Value, _>(format!("expected Response, got {other:?}"))),
    };

    let breakpoints = must_some(body.get("breakpoints").and_then(|b| b.as_array()));
    assert_eq!(breakpoints.len(), 2);
    let line_10_verified =
        breakpoints[0].get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
    let line_25_verified =
        breakpoints[1].get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
    assert!(line_10_verified, "line 10 (executable) should be verified, got {breakpoints:?}");
    assert!(line_25_verified, "line 25 (executable) should be verified, got {breakpoints:?}");
}

/// Mirrors `dispatcher::tests::test_handle_set_breakpoints_preserves_order`:
/// the response array must report breakpoints in the exact order the
/// client requested them, regardless of line value.
#[test]
fn set_breakpoints_preserves_request_order_through_dispatch() {
    let (_keep, source_path) = create_test_perl_file();
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(
        1,
        "setBreakpoints",
        Some(json!({
            "source": { "path": source_path },
            "breakpoints": [
                { "line": 25 },
                { "line": 10 },
                { "line": 15 },
            ],
        })),
    );

    let body = match response {
        DapMessage::Response { success: true, body, .. } => must_some(body),
        other => must(Err::<serde_json::Value, _>(format!(
            "expected successful Response, got {other:?}"
        ))),
    };

    let breakpoints = must_some(body.get("breakpoints").and_then(|b| b.as_array()));
    let lines: Vec<i64> =
        breakpoints.iter().filter_map(|bp| bp.get("line").and_then(|l| l.as_i64())).collect();
    assert_eq!(lines, vec![25, 10, 15], "response order must mirror request order");
}

/// Mirrors `dispatcher::tests::test_handle_set_breakpoints_missing_arguments`:
/// the dispatch handler must reject `arguments: None` with a structured
/// failure response - never panic, never return success.
#[test]
fn set_breakpoints_with_missing_arguments_fails_structured() {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(1, "setBreakpoints", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "missing arguments must produce success=false");
            assert_eq!(command, "setBreakpoints");
            assert!(message.is_some(), "missing arguments must include an error message");
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}

// --- inlineValues -------------------------------------------------------------

/// Mirrors `dispatcher::tests::test_handle_inline_values`: scanning a
/// two-line script must surface both `$x` and `$y` in the response.
///
/// `dap_comprehensive_test.rs::test_dap_inline_values` already covers a
/// similar shape; this test pins the minimal source/range that
/// `DapDispatcher`'s unit test guarded.
#[test]
fn inline_values_returns_scalars_for_two_line_script() {
    let mut file = must(NamedTempFile::with_suffix(".pl"));
    must(file.write_all(b"my $x = 1;\nmy $y = $x + 2;\n"));
    must(file.flush());
    let path = file.path().to_string_lossy().to_string();

    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(
        1,
        "inlineValues",
        Some(json!({
            "source": { "path": path },
            "startLine": 1,
            "endLine": 2,
        })),
    );

    let body = match response {
        DapMessage::Response { success: true, body, .. } => must_some(body),
        other => must(Err::<serde_json::Value, _>(format!(
            "expected successful Response, got {other:?}"
        ))),
    };

    let values = must_some(body.get("inlineValues").and_then(|v| v.as_array()));
    let saw_x =
        values.iter().any(|v| v.get("text").and_then(|t| t.as_str()).unwrap_or("").contains("$x"));
    let saw_y =
        values.iter().any(|v| v.get("text").and_then(|t| t.as_str()).unwrap_or("").contains("$y"));
    assert!(saw_x, "inlineValues must surface $x, got {values:?}");
    assert!(saw_y, "inlineValues must surface $y, got {values:?}");
}

// --- configurationDone --------------------------------------------------------

/// Documents the deliberate divergence from `DapDispatcher`:
/// `DebugAdapter::handle_configuration_done` (in
/// `crates/perl-dap/src/debug_adapter/process.rs`) does not gate on the
/// initialized state - it returns success regardless. This has been the
/// production behavior for many releases (since `DapServer::run` has only
/// ever wired `DebugAdapter` through stdio). Removing the unused
/// `DapDispatcher` strict check does not change observed behavior.
///
/// If a future change re-introduces the strict check on `DebugAdapter`,
/// this test will need to be updated alongside the issue documentation.
#[test]
fn configuration_done_before_initialize_is_permissive() {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(1, "configurationDone", None);

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(
                success,
                "DebugAdapter returns success even before initialize \
                 (DapDispatcher's strict check was unused in production)"
            );
            assert_eq!(command, "configurationDone");
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}

// --- unknown command ---------------------------------------------------------

/// Mirrors `dispatcher::tests::test_handle_unknown_command`: an unknown
/// command must return a structured failure whose message starts with
/// `"Unknown command: <name>"` - the prefix `DapDispatcher` produced and
/// that `DebugAdapter::dispatch_request` continues to produce
/// (`debug_adapter/dispatch.rs:143-149`).
#[test]
fn unknown_command_returns_unknown_command_prefix() {
    let mut adapter = DebugAdapter::new();
    let response = adapter.handle_request(42, "thisCommandDoesNotExist", None);

    match response {
        DapMessage::Response { success, command, message, request_seq, .. } => {
            assert!(!success);
            assert_eq!(command, "thisCommandDoesNotExist");
            assert_eq!(request_seq, 42);
            let message = must_some(message);
            assert!(
                message.starts_with("Unknown command: thisCommandDoesNotExist"),
                "expected `Unknown command: <name>` prefix, got: {message}"
            );
        }
        other => must(Err::<(), _>(format!("expected Response, got {other:?}"))),
    }
}

// --- sequence numbers ---------------------------------------------------------

/// Mirrors `dispatcher::tests::test_response_sequence_numbers` /
/// `test_event_sequence_numbers`: each response and each event carries a
/// strictly-monotonically-increasing `seq`.
#[test]
fn response_and_event_sequence_numbers_increase_monotonically() {
    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let r1 = adapter.handle_request(1, "initialize", None);
    let r2 = adapter.handle_request(2, "threads", None);

    let r1_seq = match r1 {
        DapMessage::Response { seq, .. } => seq,
        other => must(Err::<i64, _>(format!("expected Response, got {other:?}"))),
    };
    let r2_seq = match r2 {
        DapMessage::Response { seq, .. } => seq,
        other => must(Err::<i64, _>(format!("expected Response, got {other:?}"))),
    };
    assert!(r2_seq > r1_seq, "response seq must increase: {r1_seq} -> {r2_seq}");

    // The `initialized` event was emitted during r1; drain it and confirm
    // its seq is strictly between r1's response seq and any later activity.
    // (DebugAdapter shares one counter across responses and events.)
    let event_seq = match must(rx.recv_timeout(Duration::from_millis(500))) {
        DapMessage::Event { seq, event, .. } => {
            assert_eq!(event, "initialized");
            seq
        }
        other => must(Err::<i64, _>(format!("expected initialized Event, got {other:?}"))),
    };
    assert!(
        event_seq > r1_seq,
        "initialized event seq ({event_seq}) must be greater than its triggering response seq ({r1_seq})"
    );
}
