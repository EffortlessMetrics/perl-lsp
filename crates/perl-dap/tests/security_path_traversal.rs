use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

#[test]
fn test_launch_blocks_arbitrary_absolute_path() {
    let mut adapter = DebugAdapter::new();

    // Create a temporary file outside the current directory
    let temp_dir = tempfile::tempdir().unwrap();
    let outside_file = temp_dir.path().join("evil.pl");
    std::fs::write(&outside_file, "print 'evil'").unwrap();

    // Launch with absolute path to outside file
    let args = json!({
        "program": outside_file.to_str().unwrap(),
        "cwd": std::env::current_dir().unwrap().to_str().unwrap()
    });

    // We are mocking the "launch" request
    let response = adapter.handle_request(1, "launch", Some(args));

    if let DapMessage::Response { success, message, .. } = response {
        // Now we expect it to fail with a security error
        assert!(!success, "Launch should fail due to security validation");

        if let Some(msg) = message {
            println!("Message: {}", msg);
            assert!(msg.contains("Security Error"), "Expected Security Error, got: {}", msg);
            assert!(msg.contains("Path outside workspace") || msg.contains("Path resolves outside workspace"), "Expected path traversal/workspace error");
        } else {
            panic!("Expected error message");
        }
    } else {
        panic!("Expected response");
    }
}
