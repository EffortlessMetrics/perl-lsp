//! Integration tests for handle_pause session-presence vs signal-delivery separation.
//!
//! Regression for #898 follow-up: `handle_pause` must distinguish "no session"
//! (actionable guidance message) from "session present but signal delivery failed"
//! (accurate "Failed to pause debugger" message).
//!
//! Run with: cargo test -p perl-dap --test pause_signal_delivery_tests

use perl_dap::{DapMessage, DebugAdapter};
use perl_tdd_support::{must, must_some};

/// No active session → "no Perl debug session is active" guidance message.
///
/// This is the CORRECT behavior: when there is truly no session, the user needs
/// actionable guidance to start one.
#[test]
fn test_pause_no_session_returns_guidance_message() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(1, "pause", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "pause without session must fail");
            assert_eq!(command, "pause");
            let msg = must_some(message);
            assert!(
                msg.contains("no Perl debug session is active"),
                "no-session pause must contain the guidance message, got: {msg}"
            );
            assert!(
                !msg.contains("Failed to pause debugger"),
                "no-session pause must NOT say 'Failed to pause debugger', got: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>("Expected DapMessage::Response for pause with no session"));
            unreachable!()
        }
    }
}

/// Session present (attached pid) + signal delivery fails → "Failed to pause debugger"
/// (NOT the no-session guidance message).
///
/// This is the key regression test for #898 follow-up: before the fix, a signal
/// delivery failure on an active session caused the misleading "no Perl debug session
/// is active" guidance to fire, which is factually wrong when a session exists.
#[test]
fn test_pause_session_present_signal_failure_returns_accurate_error() {
    let mut adapter = DebugAdapter::new();
    // Seed an attached_pid that cannot receive signals (999_999 is virtually
    // guaranteed not to exist). This makes send_interrupt_signal return false
    // even though a "session" (attached pid) IS present.
    adapter.seed_attached_pid_for_test(999_999);

    let response = adapter.handle_request(1, "pause", None);

    match response {
        DapMessage::Response { success, command, message, .. } => {
            assert!(!success, "pause with signal delivery failure must fail");
            assert_eq!(command, "pause");
            let msg = must_some(message);
            // Must NOT fire the no-session guidance (the session IS present).
            assert!(
                !msg.contains("no Perl debug session is active"),
                "signal-delivery failure must NOT produce the no-session guidance, got: {msg}"
            );
            // Must produce the accurate signal-failure message.
            assert!(
                msg.contains("Failed to pause debugger"),
                "signal-delivery failure must say 'Failed to pause debugger', got: {msg}"
            );
        }
        _ => {
            must(Err::<(), _>(
                "Expected DapMessage::Response for pause with signal delivery failure",
            ));
            unreachable!()
        }
    }
}
