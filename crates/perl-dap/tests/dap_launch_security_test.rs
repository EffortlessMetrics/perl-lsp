use perl_dap::DapMessage;
use perl_dap::DebugAdapter;
use perl_tdd_support::must_some;
use serde_json::json;
use std::fs;

#[test]
fn test_launch_rejects_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();

    // Create a temporary workspace directory
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();

    // Create a file *outside* the workspace
    // We use a separate temp dir for the "system" file to ensure it's outside
    let system_temp_dir = tempfile::tempdir()?;
    let outside_script = system_temp_dir.path().join("evil.pl");
    fs::write(&outside_script, "print 'evil';")?;

    // Construct launch arguments with cwd set to workspace_root
    // and program pointing to the outside script
    let args = json!({
        "program": must_some(outside_script.to_str()),
        "cwd": must_some(workspace_root.to_str()),
        "args": []
    });

    // Handle launch request
    let response = adapter.handle_request(1, "launch", Some(args));

    // Verify response
    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Launch should have failed due to path traversal/workspace escape");
            let msg = must_some(message);
            assert!(
                msg.contains("outside your workspace folder") || msg.contains("outside workspace"),
                "Unexpected error message: {}",
                msg
            );
        }
        _ => return Err("Expected Response message".into()),
    }
    Ok(())
}

#[test]
fn test_launch_allows_valid_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();

    // Create a temporary workspace directory
    let temp_dir = tempfile::tempdir()?;
    let workspace_root = temp_dir.path().to_path_buf();

    // Create a file *inside* the workspace
    let inside_script = workspace_root.join("good.pl");
    fs::write(&inside_script, "print 'good';")?;

    // Construct launch arguments
    let args = json!({
        "program": must_some(inside_script.to_str()),
        "cwd": must_some(workspace_root.to_str()),
        "args": []
    });

    // Handle launch request
    let response = adapter.handle_request(1, "launch", Some(args));

    // Verify response
    match response {
        DapMessage::Response { success, message, .. } => {
            if !success {
                let msg = message.clone().unwrap_or_default();
                assert!(
                    !msg.contains("outside workspace") && !msg.contains("outside your workspace"),
                    "Valid path rejected by security check: {msg}"
                );
            }
        }
        _ => return Err("Expected Response message".into()),
    }
    Ok(())
}
