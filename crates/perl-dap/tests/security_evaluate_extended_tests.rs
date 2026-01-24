#![allow(clippy::unwrap_used, clippy::expect_used)]

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_evaluate_blocks_unsafe_ops() {
    let mut adapter = DebugAdapter::new();

    // List of unsafe operations that MUST be blocked in safe mode
    let unsafe_ops = [
        "eval('1+1')",
        "exit",
        "dump",
        "fork",
        "chroot('/tmp')",
        "print 'side effect'",
        "say 'side effect'",
        "printf 'side effect'",
    ];

    let mut failed_ops = Vec::new();

    for op in unsafe_ops {
        let args = json!({
            "expression": op,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        match response {
            DapMessage::Response { success, message, .. } => {
                let msg = message.unwrap_or_default();

                // Blocked = success is false AND message mentions safe mode
                if !success && msg.contains("Safe evaluation mode") {
                    println!("✓ Operation '{}' is blocked", op);
                } else {
                    println!(
                        "✗ Operation '{}' was NOT blocked (success={}, msg={})",
                        op, success, msg
                    );
                    failed_ops.push(op);
                }
            }
            _ => panic!("Expected Response"),
        }
    }

    if !failed_ops.is_empty() {
        panic!("The following unsafe ops were NOT blocked: {:?}", failed_ops);
    }
}
