use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_inline_values_path_traversal_vulnerability() {
    // 1. Setup directories
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let workspace_dir = temp_dir.path().join("workspace");
    std::fs::create_dir(&workspace_dir).expect("Failed to create workspace dir");

    // 2. Create a "secret" file OUTSIDE the workspace
    let secret_file_path = temp_dir.path().join("secret.txt");
    let mut secret_file = File::create(&secret_file_path).expect("Failed to create secret file");
    writeln!(secret_file, "my $secret_var = 'pwned';").expect("Failed to write to secret file");

    // 3. Create DebugAdapter and initialize with workspace
    let mut adapter = DebugAdapter::new();
    let init_args = json!({
        "rootPath": workspace_dir.to_str().unwrap(),
        "adapterID": "perl-dap"
    });
    adapter.handle_request(1, "initialize", Some(init_args));

    // 4. Construct inlineValues request pointing to the secret file (outside workspace)
    let args = json!({
        "frameId": 1,
        "text": "doesn't matter",
        "stoppedLocation": {
            "startLine": 1,
            "endLine": 1,
            "column": 0
        },
        "startLine": 1,
        "endLine": 2,
        "source": {
            "path": secret_file_path.to_str().unwrap()
        }
    });

    // 5. Send request
    let response = adapter.handle_request(2, "inlineValues", Some(args));

    // 6. Check response - SHOULD FAIL NOW
    match response {
        DapMessage::Response { success, message, .. } => {
            if success {
                panic!("Vulnerability NOT fixed: Successfully read file outside workspace!");
            } else {
                println!("Security fix verified: Request failed as expected.");
                let msg = message.unwrap_or_default();
                assert!(msg.contains("Security Error"), "Should report security error, got: {}", msg);
                assert!(msg.contains("Path attempts to escape workspace") || msg.contains("Path outside workspace"),
                        "Should fail with path traversal error, got: {}", msg);
            }
        }
        _ => panic!("Expected Response"),
    }
}
