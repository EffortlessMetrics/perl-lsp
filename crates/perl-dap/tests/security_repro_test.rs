use perl_dap::debug_adapter::DebugAdapter;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_launch_debugger_allows_out_of_bounds_path_repro() {
    let mut adapter = DebugAdapter::new();

    // Create a temporary directory to act as "workspace"
    let workspace = tempdir().unwrap();
    let workspace_path = workspace.path().to_path_buf();

    // Create a file OUTSIDE the workspace
    let outside_dir = tempdir().unwrap();
    let outside_script = outside_dir.path().join("malicious.pl");
    let mut file = File::create(&outside_script).unwrap();
    writeln!(file, "print 'evil';").unwrap();

    // Launch args pointing to the outside file
    let args = json!({
        "program": outside_script.to_str().unwrap(),
        "args": [],
        "cwd": workspace_path.to_str().unwrap()
    });

    // We expect this to SUCCEED currently (returning a message that it failed to launch because perl isn't installed or similar,
    // but NOT failing due to security).
    // Actually `handle_launch` returns a Response.

    let response = adapter.handle_request(1, "launch", Some(args));

    match response {
        perl_dap::DapMessage::Response { success, message, .. } => {
            println!("Launch response: success={}, message={:?}", success, message);

            assert!(!success, "Launch should fail due to security check");
            let msg = message.expect("Expected error message");
            assert!(msg.contains("Path outside workspace") || msg.contains("Security Error"), "Unexpected error message: {}", msg);
        }
        _ => panic!("Expected Response"),
    }
}
