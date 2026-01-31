use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

fn create_test_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

#[test]
fn test_evaluate_safe_mode_validation_complex() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = create_test_adapter();

    // 1. Should be BLOCKED (dangerous usage)
    let blocked_cases = [
        "each %hash",
        "keys %hash",
        "values %hash",
        "chomp $var",
        "chop $var",
        "getc",
        "read $fh, $buf, 10",
        "map { $_ } @list",
        "grep { $_ } @list",
        "sort @list",
        "system",
        "$obj->system()", // strict op, blocked even as method
        "map { system }", // block with dangerous op
        "{ map }", // ambiguous block/hash, safer to block
        "{ system }", // ambiguous block/hash, safer to block
    ];

    for op in blocked_cases {
        let args = json!({
            "expression": op,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            assert!(!success, "Op '{}' should have been BLOCKED", op);
            let msg = message.ok_or("Expected error message")?;
            if !msg.contains("not allowed") {
                 return Err(format!("Op '{}' failed but not due to security check: {}", op, msg).into());
            }
        }
    }

    // 2. Should be ALLOWED (safe usage: method calls, hash keys, fat commas)
    let allowed_cases = [
        "$obj->map(sub { $_ })",
        "$obj->keys()",
        "$hash{map}",
        "$hash{system}",
        "$hash{keys}",
        "map => 1",
        "system => 1",
    ];

    for op in allowed_cases {
        let args = json!({
            "expression": op,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(1, "evaluate", Some(args));

        if let DapMessage::Response {  message, .. } = response {
            // We expect it to FAIL due to "No debugger session" etc, but NOT "not allowed"
            if let Some(msg) = message {
                if msg.contains("not allowed") {
                     return Err(format!("Op '{}' was WRONGLY BLOCKED: {}", op, msg).into());
                }
            }
        }
    }

    Ok(())
}
