use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_security_gap_dangerous_ops_blocked() {
    let mut adapter = DebugAdapter::new();
    let seq = 1;
    let _request_seq = 1;

    // These operations should now be BLOCKED in safe evaluation mode.
    let blocked_ops = [
        "chomp $x",
        "chop $x",
        "keys %h",
        "values %h",
        "each %h",
        "getc",
        "read $fh, $buf, 10",
        "sysopen $fh, 'file', 0",
        "map { 1 } @list",
        "grep { 1 } @list",
        "sort @list",
        // Unsafe hash keys (optional args triggers call)
        "$hash{chomp}",
        "$hash{getc}",
    ];

    for op in blocked_ops {
        let args = json!({
            "expression": op,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(seq, "evaluate", Some(args));

        if let DapMessage::Response { success, message, .. } = response {
            // We expect success=false and a message starting with "Safe evaluation mode:"
            assert!(!success, "Operation '{}' should have failed", op);

            if let Some(msg) = message {
                assert!(
                    msg.starts_with("Safe evaluation mode:"),
                    "Expected Safe evaluation mode error, got: '{}' for op '{}'", msg, op
                );
            } else {
                panic!("Expected error message for op '{}'", op);
            }
        } else {
            panic!("Expected Response message");
        }
    }
}

#[test]
fn test_security_gap_harmless_ops_allowed() {
    let mut adapter = DebugAdapter::new();
    let seq = 1;
    let _request_seq = 1;

    // These should be allowed (variables, hash keys, methods)
    let allowed_ops = [
        "$map",
        "$grep",
        "$sort",
        "$keys",
        "$values",
        "$read",
        // Safe hash keys (arg required -> string)
        "$hash{map}",
        "$hash{grep}",
        "$hash{sort}",
        "$hash{keys}",
        "$hash{read}",
        // String literals
        "'map'",
        "'grep'",
        // Double quotes are considered unsafe for keywords due to interpolation risks
        // so "map" is blocked, but 'map' is allowed.
    ];

    for op in allowed_ops {
        let args = json!({
            "expression": op,
            "allowSideEffects": false
        });

        let response = adapter.handle_request(seq, "evaluate", Some(args));

        if let DapMessage::Response { message, .. } = response {
             if let Some(msg) = message {
                if msg.starts_with("Safe evaluation mode:") {
                    panic!("Harmless operation '{}' was blocked! Error: {}", op, msg);
                }
             }
        }
    }
}
