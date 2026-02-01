use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_security_gaps_are_closed() {
    let mut adapter = DebugAdapter::new();

    // List of operations that must be blocked
    let ops = vec![
        "use Data::Dumper",
        "no strict",
        "goto LABEL",
        "package Foo",
        "dump",
    ];

    for op in ops {
        let args = json!({
            "expression": op,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            let msg = message.unwrap_or_default();
            println!("Op: '{}' -> Success: {}, Message: '{}'", op, success, msg);

            // Should be success=false and message should indicate it was blocked by safe mode
            assert!(!success, "Operation '{}' should have failed", op);
            assert!(
                msg.contains("Safe evaluation mode"),
                "Operation '{}' should be blocked by Safe evaluation mode, but got: {}",
                op, msg
            );
        } else {
             panic!("Unexpected response type");
        }
    }
}
