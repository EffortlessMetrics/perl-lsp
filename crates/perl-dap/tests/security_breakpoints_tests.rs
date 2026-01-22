use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_set_breakpoints_rejects_newlines_in_condition() {
    let mut adapter = DebugAdapter::new();

    let args = json!({
        "source": { "path": "test.pl" },
        "breakpoints": [
            {
                "line": 10,
                "condition": "1\nprint 'pwned'"
            }
        ]
    });

    let response = adapter.handle_request(1, "setBreakpoints", Some(args));

    match response {
        DapMessage::Response { body, .. } => {
            let body = body.unwrap();
            let breakpoints = body.get("breakpoints").unwrap().as_array().unwrap();
            let bp = &breakpoints[0];

            // verified should be false
            assert_eq!(bp.get("verified").unwrap().as_bool(), Some(false));

            // message should indicate error about newlines
            let message = bp.get("message").and_then(|m| m.as_str()).unwrap_or("");
            assert!(
                message.contains("Condition cannot contain newlines"),
                "Expected error about newlines, got: '{}'",
                message
            );
        }
        _ => panic!("Expected Response"),
    }
}
