#[cfg(test)]
mod tests {
    use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
    use serde_json::json;

    #[test]
    fn test_security_repro_core_system_bypass() {
        let mut adapter = DebugAdapter::new();
        // This should be BLOCKED because it is CORE::
        let args = json!({
            "expression": "CORE::system('ls')",
            "context": "hover",
            "allowSideEffects": false
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            assert!(!success);
            let msg = message.unwrap_or_default();
            assert!(
                msg.contains("potentially mutating operation"),
                "CORE::system should be blocked, got: {}",
                msg
            );
        } else {
            panic!("Expected response");
        }
    }

    #[test]
    fn test_security_repro_posix_system_bypass() {
        let mut adapter = DebugAdapter::new();
        // This currently BYPASSES security because it is package qualified but not CORE::
        let args = json!({
            "expression": "POSIX::system('ls')",
            "context": "hover",
            "allowSideEffects": false
        });

        let response = adapter.handle_request(2, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            assert!(!success);
            let msg = message.unwrap_or_default();

            // If the vulnerability exists, this will fail with "No debugger session"
            // If fixed, it should fail with "potentially mutating operation"
            if msg.contains("No debugger session") {
                panic!("VULNERABILITY CONFIRMED: POSIX::system was allowed in safe evaluation mode!");
            }

            assert!(
                msg.contains("potentially mutating operation"),
                "POSIX::system should be blocked, got: {}",
                msg
            );
        } else {
            panic!("Expected response");
        }
    }

    #[test]
    fn test_security_repro_use_bypass() {
        let mut adapter = DebugAdapter::new();
        // 'use' is not in the dangerous ops list
        let args = json!({
            "expression": "use Socket; 1",
            "context": "hover",
            "allowSideEffects": false
        });

        let response = adapter.handle_request(3, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            assert!(!success);
            let msg = message.unwrap_or_default();

             if msg.contains("No debugger session") {
                panic!("VULNERABILITY CONFIRMED: 'use' was allowed in safe evaluation mode!");
            }

            assert!(
                msg.contains("potentially mutating operation"),
                "'use' should be blocked, got: {}",
                msg
            );
        }
    }
}
