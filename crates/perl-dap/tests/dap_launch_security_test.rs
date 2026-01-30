use perl_dap::DapMessage;
use perl_dap::DebugAdapter;
use serde_json::json;
use std::fs;

#[test]
fn test_launch_rejects_path_traversal() {
    let mut adapter = DebugAdapter::new();

    // Create a temporary workspace directory
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let workspace_root = temp_dir.path().to_path_buf();

    // Create a file *outside* the workspace
    // We use a separate temp dir for the "system" file to ensure it's outside
    let system_temp_dir = tempfile::tempdir().expect("Failed to create system temp dir");
    let outside_script = system_temp_dir.path().join("evil.pl");
    fs::write(&outside_script, "print 'evil';").expect("Failed to write evil script");

    // Construct launch arguments with cwd set to workspace_root
    // and program pointing to the outside script
    let args = json!({
        "program": outside_script.to_str().unwrap(),
        "cwd": workspace_root.to_str().unwrap(),
        "args": []
    });

    // Handle launch request
    let response = adapter.handle_request(1, "launch", Some(args));

    // Verify response
    match response {
        DapMessage::Response { success, message, .. } => {
            if success {
                panic!("Launch should have failed due to path traversal/workspace escape");
            } else {
                let msg = message.unwrap();
                assert!(msg.contains("Security check failed"), "Unexpected error message: {}", msg);
                assert!(msg.contains("outside workspace"), "Error should indicate path is outside workspace");
            }
        }
        _ => panic!("Expected Response message"),
    }
}

#[test]
fn test_launch_allows_valid_path() {
    let mut adapter = DebugAdapter::new();

    // Create a temporary workspace directory
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let workspace_root = temp_dir.path().to_path_buf();

    // Create a file *inside* the workspace
    let inside_script = workspace_root.join("good.pl");
    fs::write(&inside_script, "print 'good';").expect("Failed to write good script");

    // Construct launch arguments
    let args = json!({
        "program": inside_script.to_str().unwrap(),
        "cwd": workspace_root.to_str().unwrap(),
        "args": []
    });

    // Handle launch request
    let response = adapter.handle_request(1, "launch", Some(args));

    // Verify response
    match response {
        DapMessage::Response { success, message, .. } => {
            if !success {
                let msg = message.clone().unwrap_or_default();
                if msg.contains("outside workspace") || msg.contains("Security check failed") {
                    panic!("Valid path rejected by security check: {}", msg);
                }
            }
        }
        _ => panic!("Expected Response message"),
    }
}
