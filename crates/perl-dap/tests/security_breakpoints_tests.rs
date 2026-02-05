//! Security tests for breakpoint handling
//!
//! Tests for preventing protocol injection vulnerabilities in breakpoint conditions.

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_set_breakpoints_rejects_newlines_in_condition() -> TestResult {
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

            let body = body.ok_or("Response should have body")?;
            let breakpoints = body
                .get("breakpoints")
                .and_then(|b| b.as_array())
                .ok_or("Body should have breakpoints array")?;

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
                message, "Expression cannot contain newlines",
                "Condition with newline was not rejected by validation logic (Risk of protocol injection)"
            );
        }
        _ => return Err("Expected Response message".into()),
    }
    Ok(())
}

#[test]
fn test_set_breakpoints_rejects_carriage_returns_in_condition() -> TestResult {
    let mut adapter = DebugAdapter::new();

    // Test with carriage return (Windows-style line ending attack)
    let args = json!({
        "source": { "path": "test.pl" },
        "breakpoints": [
            {
                "line": 10,
                "condition": "1\rprint 'hacked'"
            }
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(args));

    match response {
        DapMessage::Response { success, body, .. } => {
            assert!(success, "Request should succeed (even if breakpoint is not verified)");

            let body = body.ok_or("Response should have body")?;
            let breakpoints = body
                .get("breakpoints")
                .and_then(|b| b.as_array())
                .ok_or("Body should have breakpoints array")?;

            assert_eq!(breakpoints.len(), 1);
            let bp = &breakpoints[0];

            let verified = bp.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
            let message = bp.get("message").and_then(|m| m.as_str()).unwrap_or("");

            assert!(!verified, "Breakpoint with carriage return should not be verified");
            assert_eq!(
                message, "Expression cannot contain newlines",
                "Condition with carriage return was not rejected"
            );
        }
        _ => return Err("Expected Response message".into()),
    }
    Ok(())
}
