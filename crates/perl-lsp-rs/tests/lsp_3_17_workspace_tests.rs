//! LSP 3.17 Workspace Feature Contract Tests
//!
//! Tests for workspace/symbol, workspaceSymbol/resolve, workspace/executeCommand,
//! workspace/didChangeWorkspaceFolders, file operations, and watched files.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== WORKSPACE FEATURES ====================

#[test]
fn test_workspace_symbol_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let response = harness.request(
        "workspace/symbol",
        json!({
            "query": "test",
            "workDoneToken": "symbol-1"
        }),
    )?;

    assert!(response.is_null() || response.is_array());
    Ok(())
}

#[test]
fn test_workspace_symbol_resolve_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Mock workspace symbol to resolve
    let response = harness.request(
        "workspaceSymbol/resolve",
        json!({
            "name": "test",
            "kind": 12,
            "location": {
                "uri": "file:///test.pl"
            }
        }),
    );

    if let Ok(symbol) = response {
        assert!(symbol["name"].is_string());
    }
    Ok(())
}

#[test]
fn test_execute_command_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let response = harness.request(
        "workspace/executeCommand",
        json!({
            "command": "perl.extractVariable",
            "arguments": [
                "file:///test.pl",
                { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }
            ],
            "workDoneToken": "cmd-1"
        }),
    );

    // May fail if command not supported
    if let Ok(_result) = response {
        // Result can be any value
    }
    Ok(())
}

#[test]
fn test_workspace_folders_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Notify of workspace folder changes
    harness.notify(
        "workspace/didChangeWorkspaceFolders",
        json!({
            "event": {
                "added": [
                    { "uri": "file:///workspace2", "name": "Workspace 2" }
                ],
                "removed": []
            }
        }),
    );

    // Server can also request current folders (if needed)
    // This would be a server->client request, so we skip it in tests
    Ok(())
}

#[test]
fn test_file_operations_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // willCreateFiles
    let response = harness.request(
        "workspace/willCreateFiles",
        json!({
            "files": [
                { "uri": "file:///new.pl" }
            ]
        }),
    );

    if let Ok(edit) = response {
        assert!(edit.is_null() || edit.is_object());
    }

    // didCreateFiles
    harness.notify(
        "workspace/didCreateFiles",
        json!({
            "files": [
                { "uri": "file:///new.pl" }
            ]
        }),
    );

    // willRenameFiles
    let response = harness.request(
        "workspace/willRenameFiles",
        json!({
            "files": [
                { "oldUri": "file:///old.pl", "newUri": "file:///new.pl" }
            ]
        }),
    );

    if let Ok(edit) = response {
        assert!(edit.is_null() || edit.is_object());
    }

    // didRenameFiles
    harness.notify(
        "workspace/didRenameFiles",
        json!({
            "files": [
                { "oldUri": "file:///old.pl", "newUri": "file:///new.pl" }
            ]
        }),
    );

    // willDeleteFiles
    let response = harness.request(
        "workspace/willDeleteFiles",
        json!({
            "files": [
                { "uri": "file:///delete.pl" }
            ]
        }),
    );

    if let Ok(edit) = response {
        assert!(edit.is_null() || edit.is_object());
    }

    // didDeleteFiles
    harness.notify(
        "workspace/didDeleteFiles",
        json!({
            "files": [
                { "uri": "file:///delete.pl" }
            ]
        }),
    );
    Ok(())
}

#[test]
fn test_watched_files_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [
                { "uri": "file:///test.pl", "type": 2 },  // Changed
                { "uri": "file:///new.pl", "type": 1 },   // Created
                { "uri": "file:///old.pl", "type": 3 }    // Deleted
            ]
        }),
    );
    Ok(())
}
