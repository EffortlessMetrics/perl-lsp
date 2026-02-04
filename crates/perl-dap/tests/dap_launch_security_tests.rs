#[cfg(test)]
mod tests {
    use perl_dap::debug_adapter::{DebugAdapter, DapMessage};
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_security_gap_launch_outside_workspace() {
        // Setup: Create a workspace and a file OUTSIDE of it
        let root_dir = TempDir::new().unwrap();
        let workspace_dir = root_dir.path().join("workspace");
        std::fs::create_dir(&workspace_dir).unwrap();

        let outside_dir = root_dir.path().join("outside");
        std::fs::create_dir(&outside_dir).unwrap();

        let secret_file = outside_dir.join("secret.pl");
        let mut f = File::create(&secret_file).unwrap();
        writeln!(f, "print 'I should not run';").unwrap();

        // Initialize DebugAdapter
        let mut adapter = DebugAdapter::new();

        // Mock launch arguments
        // We pretend "workspace_dir" is our cwd/workspace
        // But we try to launch "secret_file" which is outside
        let args = json!({
            "program": secret_file.to_str().unwrap(),
            "cwd": workspace_dir.to_str().unwrap(),
            "args": []
        });

        // Execute handle_launch
        // Since we can't easily capture the spawned process behavior in this unit test without mocking Command,
        // we check if it returns Success (meaning it accepted the path).
        // Real security fix would return success: false with an error message.

        let response = adapter.handle_request(1, "launch", Some(args));

        match response {
            DapMessage::Response { success, message, .. } => {
                assert!(!success, "Launch should have failed due to security check");

                let msg = message.expect("Expected failure message");
                println!("Got expected error: {}", msg);
                assert!(
                    msg.contains("Path outside workspace") || msg.contains("Path traversal") || msg.contains("Security Error"),
                    "Unexpected error message: {}", msg
                );
            }
            _ => panic!("Expected Response"),
        }
    }
}
