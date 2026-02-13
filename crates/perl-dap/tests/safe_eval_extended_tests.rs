
use perl_dap::debug_adapter::{DebugAdapter, DapMessage};
use serde_json::json;

#[test]
fn test_safe_eval_repro_sysopen_blocked() {
    let mut adapter = DebugAdapter::new();
    let request_seq = 1;

    // Request with allowSideEffects: false (safe mode)
    // sysopen is dangerous as it can create/overwrite files
    let args = json!({
        "expression": "sysopen($fh, 'file.txt', 0x0200)",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(request_seq, "evaluate", Some(args));

    if let DapMessage::Response { success: _, message, .. } = response {
        if let Some(msg) = message {
            if msg.contains("Safe evaluation mode") {
                // Correctly blocked
            } else if msg.contains("No debugger session") {
                panic!("sysopen passed security check (reached session check) but should be blocked");
            } else {
                panic!("sysopen failed with unexpected error: {}", msg);
            }
        } else {
            // If success is true and no message, it definitely passed security check
            panic!("sysopen passed security check (success=true) but should be blocked");
        }
    } else {
        panic!("Unexpected response type");
    }
}

#[test]
fn test_safe_eval_repro_chomp_blocked() {
    let mut adapter = DebugAdapter::new();
    let request_seq = 1;

    // chomp mutates the string in place
    let args = json!({
        "expression": "chomp($str)",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(request_seq, "evaluate", Some(args));

    if let DapMessage::Response { success: _, message, .. } = response {
        if let Some(msg) = message {
            if msg.contains("Safe evaluation mode") {
                // Correctly blocked
            } else if msg.contains("No debugger session") {
                panic!("chomp passed security check (reached session check) but should be blocked");
            } else {
                panic!("chomp failed with unexpected error: {}", msg);
            }
        } else {
            panic!("chomp passed security check (success=true) but should be blocked");
        }
    }
}

#[test]
fn test_safe_eval_repro_package_blocked() {
    let mut adapter = DebugAdapter::new();
    let request_seq = 1;

    // package changes the current package context
    let args = json!({
        "expression": "package Foo;",
        "allowSideEffects": false
    });

    let response = adapter.handle_request(request_seq, "evaluate", Some(args));

    if let DapMessage::Response { success: _, message, .. } = response {
        if let Some(msg) = message {
            if msg.contains("Safe evaluation mode") {
                // Correctly blocked
            } else if msg.contains("No debugger session") {
                panic!("package passed security check (reached session check) but should be blocked");
            } else {
                panic!("package failed with unexpected error: {}", msg);
            }
        } else {
            panic!("package passed security check (success=true) but should be blocked");
        }
    }
}
