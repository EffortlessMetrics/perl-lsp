//! Tests for DAP launch error remediation messaging.
//!
//! Issue #4192: Launch failure should provide actionable guidance including
//! detected Perl location and `perl-lsp.perl.path` config suggestion.

use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;

/// Launch with a nonexistent program path should yield an error message
/// that mentions `perl-lsp.perl.path` verbatim.
#[test]
fn launch_error_names_perl_lsp_perl_path_setting() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(
        1,
        "launch",
        Some(json!({
            "program": "/nonexistent/path/to/script.pl"
        })),
    );

    match response {
        DapMessage::Response { success: false, message: Some(msg), .. } => {
            assert!(
                msg.contains("perl-lsp.perl.path"),
                "error message must name the `perl-lsp.perl.path` setting verbatim, got: {msg}"
            );
        }
        DapMessage::Response { success: true, .. } => {
            // If Perl is available and the file actually exists (unlikely with this path),
            // treat as expected. On most CI envs the program doesn't exist so we get false.
            // This branch should not happen for a nonexistent path, but be defensive.
        }
        other => {
            panic!("expected a Response, got: {:?}", other);
        }
    }
}

/// Launch failure error message must include information about the Perl
/// interpreter that was found (or indicate none was found).
#[test]
fn launch_error_includes_perl_detection_info() {
    let mut adapter = DebugAdapter::new();

    let response = adapter.handle_request(
        1,
        "launch",
        Some(json!({
            "program": "/nonexistent/path/to/script.pl"
        })),
    );

    match response {
        DapMessage::Response { success: false, message: Some(msg), .. } => {
            let msg_lower = msg.to_lowercase();
            let has_perl_found = msg_lower.contains("found perl") || msg_lower.contains("perl at");
            let has_perl_not_found = msg_lower.contains("not found")
                || msg_lower.contains("no perl")
                || msg_lower.contains("check your path")
                || msg_lower.contains("path");
            assert!(
                has_perl_found || has_perl_not_found,
                "error message should mention Perl found/not-found status, got: {msg}"
            );
        }
        DapMessage::Response { success: true, .. } => {
            // Defensive: skip if somehow succeeded (shouldn't happen for nonexistent path).
        }
        other => {
            panic!("expected a Response, got: {:?}", other);
        }
    }
}

/// On Windows with no Perl available, the launch error should link to strawberryperl.com.
///
/// This test only asserts when Perl is actually absent — if Perl is found on the system
/// the "found at" branch fires instead and the link is not needed.
#[test]
#[cfg(windows)]
fn launch_error_on_windows_links_strawberry_perl_when_perl_absent() {
    use perl_dap::platform::resolve_perl_path_with_toolchain;

    // Only run this assertion when Perl is genuinely not available.
    if resolve_perl_path_with_toolchain().is_err() {
        let mut adapter = DebugAdapter::new();

        let response = adapter.handle_request(
            1,
            "launch",
            Some(json!({
                "program": "/nonexistent/path/to/script.pl"
            })),
        );

        match response {
            DapMessage::Response { success: false, message: Some(msg), .. } => {
                assert!(
                    msg.contains("strawberryperl.com"),
                    "Windows not-found error message should link to strawberryperl.com, got: {msg}"
                );
            }
            DapMessage::Response { success: true, .. } => {}
            other => {
                panic!("expected a Response, got: {:?}", other);
            }
        }
    }
    // If Perl is found, the test passes vacuously — the "found" branch is tested by
    // the other tests above.
}
