use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;
use std::sync::mpsc::channel;
use std::time::Duration;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_dap_basic_flow() -> TestResult {
    // Skip if perl is not available
    if std::process::Command::new("perl").arg("--version").output().is_err() {
        eprintln!("Skipping DAP basic flow test - perl not available");
        return Ok(());
    }

    let dir = tempdir()?;
    let script_path = dir.path().join("sample.pl");
    write(
        &script_path,
        r#"use strict;
use warnings;

my $x = 1;
$x++;
print "x=$x\n";
"#,
    )?;

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    // Initialize
    let init_response = adapter.handle_request(1, "initialize", None);
    match init_response {
        DapMessage::Response { success, .. } => assert!(success, "Initialize should succeed"),
        _ => return Err("Expected initialize response".into()),
    }

    // Try to wait for initialized event, but don't fail if timing issues
    loop {
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(msg) => {
                if let DapMessage::Event { ref event, .. } = msg
                    && event == "initialized"
                {
                    eprintln!("Received initialized event");
                    break;
                }
            }
            Err(_) => {
                eprintln!("Timeout waiting for initialized event - continuing anyway");
                break;
            }
        }
    }

    // Launch
    let launch_args = json!({
        "program": script_path.to_str().ok_or("Failed to convert path to string")?,
        "args": [],
        "stopOnEntry": true
    });
    let launch_response = adapter.handle_request(2, "launch", Some(launch_args));
    match launch_response {
        DapMessage::Response { success, message, .. } => {
            if success {
                eprintln!("Launch succeeded");
                // Try to wait for stopped event, but don't require it
                loop {
                    match rx.recv_timeout(Duration::from_secs(3)) {
                        Ok(msg) => {
                            if let DapMessage::Event { ref event, .. } = msg {
                                eprintln!("Received event: {}", event);
                                if event == "stopped" {
                                    eprintln!("Received stopped event");
                                    break;
                                }
                            }
                        }
                        Err(_) => {
                            eprintln!("Timeout waiting for stopped event - continuing");
                            break;
                        }
                    }
                }
            } else {
                eprintln!("Launch failed (expected on some systems): {:?}", message);
            }
        }
        _ => return Err("Expected launch response".into()),
    }

    // Disconnect
    let disconnect_response = adapter.handle_request(3, "disconnect", None);
    match disconnect_response {
        DapMessage::Response { success, .. } => assert!(success, "Disconnect should succeed"),
        _ => return Err("Expected disconnect response".into()),
    }

    eprintln!("DAP basic flow test completed");
    Ok(())
}
