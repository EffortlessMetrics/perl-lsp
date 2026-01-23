use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::io::Write;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread;

#[test]
fn test_breakpoint_condition_injection() {
    // 1. Setup a dummy perl script
    let mut script_file = tempfile::NamedTempFile::new().unwrap();
    write!(script_file, "print 'Hello';\n").unwrap();
    let script_path = script_file.path().to_str().unwrap().to_string();

    // 2. Start adapter
    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    // 3. Launch debugger
    let args = json!({
        "program": script_path,
        "args": [],
        "stopOnEntry": true
    });

    let response = adapter.handle_request(1, "launch", Some(args));
    if let DapMessage::Response { success, message, .. } = response {
        if !success {
            panic!("Launch failed: {:?}", message);
        }
    }

    // Wait for stopped event (entry)
    let mut stopped = false;
    // We need to process events
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Ok(msg) = rx.try_recv() {
            if let DapMessage::Event { event, .. } = msg {
                if event == "stopped" {
                    stopped = true;
                    break;
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(stopped, "Debugger did not stop on entry");

    // 4. Send setBreakpoints with malicious condition
    // Inject a command to create a file
    let pwn_path = script_path.replace(".tmp", ".pwned"); // Derive a path
    let injection_payload = format!("1\nopen(my $f, '>', '{}'); print $f 'pwned'; close($f);", pwn_path);

    let bp_args = json!({
        "source": { "path": script_path },
        "breakpoints": [
            {
                "line": 1,
                "condition": injection_payload
            }
        ]
    });

    let response = adapter.handle_request(2, "setBreakpoints", Some(bp_args));

    // Clean up - disconnect to kill process
    let _ = adapter.handle_request(3, "disconnect", None);

    // Check response
    if let DapMessage::Response { success, body, .. } = response {
        assert!(success, "Request should succeed, but individual breakpoint should be rejected");

        if let Some(b) = body {
            let bps = b.get("breakpoints").and_then(|v| v.as_array()).expect("Should have breakpoints array");
            let bp = &bps[0];
            let verified = bp.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
            let message = bp.get("message").and_then(|m| m.as_str()).unwrap_or("");

            assert!(!verified, "Breakpoint should be rejected (not verified)");
            assert!(message.contains("newline") || message.contains("invalid") || message.contains("Condition cannot contain newlines"),
                    "Message should explain rejection: {}", message);
        }
    } else {
        panic!("Expected Response");
    }
}
