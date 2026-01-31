//! Security regression tests for legacy executeCommand handlers (perl.runTest, perl.runTestFile)
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn setup_server(root_path: Option<String>) -> LspServer {
    let mut server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootPath": root_path,
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    let _response = server.handle_request(init_request);

    // Send the initialized notification to complete the handshake
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialized".to_string(),
        params: Some(json!({})),
        id: None,
    };

    let _initialized_response = server.handle_request(initialized_request);
    server
}

#[test]
fn test_legacy_run_test_file_path_traversal() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_dir = TempDir::new()?;
    let workspace_path = workspace_dir.path().to_string_lossy().to_string();

    // Create a file OUTSIDE the workspace
    let outside_dir = TempDir::new()?;
    let outside_file = outside_dir.path().join("outside.pl");
    fs::write(&outside_file, "print 'OUTSIDE_EXECUTED';")?;
    let outside_path_str = outside_file.to_string_lossy().to_string();

    let mut server = setup_server(Some(workspace_path.clone()));

    let uri = format!("file://{}", outside_path_str);

    // Open the file in the server (required for legacy commands to work)
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "print 'OUTSIDE_EXECUTED';"
            }
        })),
        id: None,
    };

    let _ = server.handle_request(open_request);

    // Execute the legacy runTestFile command with the outside file
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.runTestFile",
            "arguments": [uri]
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(execute_request).ok_or("No response")?;

    // VERIFY FIX: Expect an error about path traversal

    if let Some(error) = response.error {
        if error.message.contains("Path traversal") || error.message.contains("outside workspace") {
             // Success: Security check blocked execution
             return Ok(());
        }
        panic!("Unexpected error: {}", error.message);
    }

    panic!("Vulnerability STILL EXIST: Command executed successfully");
}
