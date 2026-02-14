use perl_dap::DebugAdapter;
use serde_json::json;
use std::fs;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_security_repro_arbitrary_file_read() {
    // Setup: Create a sensitive file outside the "workspace"
    let temp_dir = tempdir().expect("failed to create temp dir");
    let secret_file = temp_dir.path().join("secret.txt");
    let secret_content = "SENSITIVE_DATA_DO_NOT_LEAK";
    let mut file = fs::File::create(&secret_file).expect("failed to create secret file");
    file.write_all(secret_content.as_bytes()).expect("failed to write secret");

    // Create "workspace" subdirectory
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir(&workspace).expect("failed to create workspace");
    let main_pl = workspace.join("main.pl");
    let mut file = fs::File::create(&main_pl).expect("failed to create main.pl");
    file.write_all(b"print 'hello';").expect("failed to write main.pl");

    // Initialize adapter
    let mut adapter = DebugAdapter::new();

    // Simulate initialize request (currently ignored, but part of protocol)
    let init_args = json!({
        "rootPath": workspace.to_str().unwrap()
    });
    adapter.handle_request(1, "initialize", Some(init_args));

    // Attack: Try to read the secret file using inlineValues request
    // The path is outside the workspace!
    let args = json!({
        "frameId": 1,
        "text": "",
        "stoppedLocation": {
            "startLine": 1,
            "endLine": 1,
            "column": 1
        },
        "source": {
            "path": secret_file.to_str().unwrap()
        },
        "startLine": 1,
        "endLine": 1
    });

    let response = adapter.handle_request(2, "inlineValues", Some(args));

    match response {
        perl_dap::DapMessage::Response { success, body, message, .. } => {
            if success {
                // If successful, check if we leaked the content
                if let Some(body) = body {
                    println!("Response Body: {}", body);
                    // inlineValues returns variable values found in the text.
                    // If the text is just "SENSITIVE_DATA_DO_NOT_LEAK", it might not find any variables.
                    // But if we can read the file, it means we bypassed security.
                    // The function `collect_inline_values` parses the content.

                    // However, just the fact that it succeeded (success: true) means it read the file!
                    // If it failed to read, it would return success: false with "Failed to read source file".
                    panic!("VULNERABILITY CONFIRMED: Successfully accessed file outside workspace!");
                }
            } else {
                // If failed, check message
                println!("Request failed as expected: {:?}", message);
            }
        }
        _ => panic!("Unexpected response type"),
    }
}
