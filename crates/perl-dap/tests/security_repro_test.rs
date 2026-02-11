use perl_dap::DapMessage;
use perl_dap::DebugAdapter;
use serde_json::json;

#[test]
fn test_security_repro_core_system_bypass() {
    let mut adapter = DebugAdapter::new();

    // We try to evaluate CORE'system("ls") which bypasses the current check
    // because it treats ' as a string delimiter due to is_in_single_quotes logic
    // seeing ' preceded by 'E' (part of CORE).

    let args = json!({
        "expression": "CORE'system(\"ls\")",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(1, "evaluate", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        // success should be false regardless (either security error or no session error)
        assert!(!success);

        let msg = message.expect("Expected message in response");

        // If the vulnerability exists, the message will be "No debugger session"
        // If fixed, it will be "Safe evaluation mode: ..."
        assert!(
            msg.contains("Safe evaluation mode"),
            "Security check bypassed! The expression was allowed to proceed to session check. Got message: '{}'",
            msg
        );
    } else {
        panic!("Expected response");
    }
}
