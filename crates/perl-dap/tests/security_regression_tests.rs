use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

#[test]
fn test_command_injection_via_program_argument() {
    // This test attempts to reproduce the vulnerability where passing "-e" as the program
    // allows executing arbitrary Perl code via arguments.
    // We use a BEGIN block because the debugger stops before executing normal code,
    // but BEGIN blocks run during compilation.

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": "-e",
        "args": ["BEGIN { print \"pwned\\n\"; } exit;"]
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    // Verify response indicates failure (due to file "-e" not found)
    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail because file '-e' does not exist");
            assert!(
                message.unwrap().contains("does not exist"),
                "Should fail with file not found error"
            );
        }
        _ => panic!("Expected Response"),
    }

    // Give it a moment to potentially run and produce output (if vulnerable)
    thread::sleep(Duration::from_millis(500));

    // Check if we received any output containing "pwned"
    let mut found_pwned = false;
    while let Ok(msg) = rx.try_recv() {
        if let DapMessage::Event { event, body, .. } = msg {
            if event == "output" {
                if let Some(body) = body {
                    if let Some(output) = body.get("output").and_then(|o| o.as_str()) {
                        if output.contains("pwned") {
                            found_pwned = true;
                        }
                    }
                }
            }
        }
    }

    if found_pwned {
        println!("VULNERABILITY REPRODUCED: Code execution confirmed via -e injection");
    } else {
        println!("Vulnerability not reproduced (safe)");
    }

    // Assert that we don't see "pwned" (this should pass after fix)
    assert!(!found_pwned, "Should not execute arbitrary code via -e");
}

#[test]
fn test_launch_with_nonexistent_file_errors_gracefully() {
    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": "nonexistent_file_12345.pl",
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should fail for nonexistent file");
            assert!(message.unwrap().contains("does not exist"), "Should return meaningful error");
        }
        _ => panic!("Expected Response"),
    }
}
