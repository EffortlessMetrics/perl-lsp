//! Security tests for breakpoint handling
//!
//! Tests for preventing protocol injection vulnerabilities in breakpoint conditions.

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_set_breakpoints_rejects_newlines_in_condition() {
    let mut adapter = DebugAdapter::new();

    // Construct arguments with a malicious condition containing a newline
    // This simulates an attempt to inject commands into the Perl debugger protocol
    let args = json!({
        "source": { "path": "test.pl" },
        "breakpoints": [
            {
                "line": 10,
                // In a vulnerable implementation, this newline allows injecting a new command
                "condition": "1; print \"PWNED\"\n"
            }
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(args));

    match response {
        DapMessage::Response { success, body, .. } => {
            assert!(success, "Request should succeed (even if breakpoint is not verified)");

            let body = body.expect("Response should have body");
            let breakpoints = body.get("breakpoints")
                .and_then(|b| b.as_array())
                .expect("Body should have breakpoints array");

            assert_eq!(breakpoints.len(), 1);
            let bp = &breakpoints[0];

            let verified = bp.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
            let message = bp.get("message").and_then(|m| m.as_str()).unwrap_or("");

            // Verify that the breakpoint is NOT verified
            assert!(!verified, "Breakpoint with malicious condition should not be verified");

            println!("Breakpoint verification message: {}", message);

            // Strictly assert that the security validation caught the newline
            // This ensures that if the validation is removed, the test will fail (regression test)
            assert_eq!(
                message,
                "Breakpoint condition cannot contain newlines",
                "Condition with newline was not rejected by validation logic (Risk of protocol injection)"
            );
        },
        _ => panic!("Expected Response message"),
    }
}
