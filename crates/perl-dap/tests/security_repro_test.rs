use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::sync::mpsc::channel;
use std::fs;

#[test]
fn test_launch_outside_workspace_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the workspace
    let workspace_dir = tempfile::tempdir()?;
    let workspace_path = workspace_dir.path().to_path_buf();

    // Create a temporary directory for the "outside" world
    let outside_dir = tempfile::tempdir()?;
    let outside_script = outside_dir.path().join("malicious.pl");

    // Create a dummy script
    fs::write(&outside_script, "print 'I should not run';")?;

    let mut adapter = DebugAdapter::new();
    let (tx, _rx) = channel();
    adapter.set_event_sender(tx);

    let args = json!({
        "program": outside_script.to_str().unwrap(),
        "cwd": workspace_path.to_str().unwrap(),
        "args": []
    });

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        DapMessage::Response { success, message, .. } => {
            if success {
                println!("Launch succeeded (VULNERABILITY REPRODUCED: Path outside workspace was allowed)");
            }
            assert!(!success, "Launch should fail for path outside workspace");

            let msg = message.ok_or("Expected error message")?;
            assert!(
                msg.contains("Path outside workspace") || msg.contains("Path attempts to escape"),
                "Should return security error, got: {}",
                msg
            );
        }
        _ => return Err("Expected Response".into()),
    }

    Ok(())
}
