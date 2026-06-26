use perl_dap::DapMessage;
use perl_dap::DebugAdapter;
use perl_tdd_support::must_some;
use serde_json::json;
use std::fs;

fn initialize_adapter(adapter: &mut DebugAdapter) {
    let response = adapter.handle_request(1, "initialize", None);
    assert!(
        matches!(response, DapMessage::Response { success: true, .. }),
        "initialize should succeed before launch security checks, got: {response:?}"
    );
}

#[test]
fn test_launch_accepts_cwd_independently_of_script_location()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    initialize_adapter(&mut adapter);

    // Create separate directories for the script and the working directory
    let scripts_dir = tempfile::tempdir()?;
    let work_dir = tempfile::tempdir()?;

    let script_path = scripts_dir.path().join("program.pl");
    fs::write(&script_path, "print 'hello';")?;

    // Launch with cwd set to a different directory than where the script is located
    // This should succeed - cwd is the execution directory, not the workspace boundary
    let args = json!({
        "program": must_some(script_path.to_str()),
        "cwd": must_some(work_dir.path().to_str()),
        "args": []
    });

    // Handle launch request (seq 2; initialize used seq 1)
    let response = adapter.handle_request(2, "launch", Some(args));

    // Verify response
    match response {
        DapMessage::Response { success, message, .. } => {
            // The launch should succeed because both script and cwd are valid paths
            if !success {
                // Some failures are ok (e.g., perl not found), but not workspace-related failures
                let msg = message.unwrap_or_default();
                assert!(
                    !msg.contains("outside your workspace") && !msg.contains("outside workspace"),
                    "Unexpected workspace rejection with valid paths: {}",
                    msg
                );
            }
        }
        _ => return Err("Expected Response message".into()),
    }
    Ok(())
}

#[test]
fn test_launch_allows_valid_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    initialize_adapter(&mut adapter);

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
    let response = adapter.handle_request(2, "launch", Some(args));

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
