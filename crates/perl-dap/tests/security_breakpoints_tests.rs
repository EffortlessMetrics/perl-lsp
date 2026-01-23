use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::sync::mpsc::channel;

#[test]
fn test_breakpoint_condition_newline_injection() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "source": { "path": "test.pl" },
        "breakpoints": [
            {
                "line": 10,
                // We inject a literal newline character to simulate a command injection attempt.
                // This would split the command sent to the debugger into multiple lines.
                "condition": "1\nprint \"INJECTED\""
            }
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(args));

    match response {
        DapMessage::Response { success, body, .. } => {
            assert!(success, "Request should succeed but return unverified breakpoint");
            let body = body.unwrap();
            let breakpoints = body.get("breakpoints").unwrap().as_array().unwrap();
            let bp = &breakpoints[0];

            let message = bp.get("message").and_then(|m| m.as_str()).unwrap_or("");

            // We expect the validation to reject the newline
            if !message.contains("newlines") {
                 panic!("Security vulnerability: Breakpoint condition with newlines was not rejected. Message: {}", message);
            }
        }
        _ => panic!("Expected Response"),
    }
}
