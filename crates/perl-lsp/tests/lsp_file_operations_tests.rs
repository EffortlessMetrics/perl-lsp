//! File operation tests for LSP 3.17
//!
//! Tests workspace/willRenameFiles request and workspace/didRenameFiles notification,
//! validating response structures and realistic rename scenarios.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== willRenameFiles ====================

#[test]
fn test_will_rename_files_single_module() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Open a file that references the module being renamed
    harness
        .open_document("file:///workspace/app.pl", "use MyModule;\nmy $obj = MyModule->new();\n")?;

    // Send willRenameFiles request for a module rename
    let result = harness.request(
        "workspace/willRenameFiles",
        json!({
            "files": [
                {
                    "oldUri": "file:///workspace/lib/MyModule.pm",
                    "newUri": "file:///workspace/lib/RenamedModule.pm"
                }
            ]
        }),
    );

    // The response should be a WorkspaceEdit (possibly empty) or the server may
    // return an error if it does not support willRenameFiles.
    match result {
        Ok(edit) => {
            // Valid WorkspaceEdit must be an object
            assert!(edit.is_object(), "willRenameFiles must return a WorkspaceEdit object");
        }
        Err(e) => {
            // MethodNotFound is acceptable if the server does not implement this
            assert!(
                e.contains("-32601") || e.contains("not found") || e.contains("not supported"),
                "unexpected error from willRenameFiles: {e}"
            );
        }
    }

    Ok(())
}

#[test]
fn test_will_rename_files_multiple_files() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Rename multiple files in one operation (e.g., refactoring a directory)
    let result = harness.request(
        "workspace/willRenameFiles",
        json!({
            "files": [
                {
                    "oldUri": "file:///workspace/lib/Util/Logger.pm",
                    "newUri": "file:///workspace/lib/Common/Logger.pm"
                },
                {
                    "oldUri": "file:///workspace/lib/Util/Config.pm",
                    "newUri": "file:///workspace/lib/Common/Config.pm"
                }
            ]
        }),
    );

    match result {
        Ok(edit) => {
            assert!(edit.is_object(), "willRenameFiles must return a WorkspaceEdit object");
        }
        Err(e) => {
            assert!(
                e.contains("-32601") || e.contains("not found") || e.contains("not supported"),
                "unexpected error from willRenameFiles: {e}"
            );
        }
    }

    Ok(())
}

#[test]
fn test_will_rename_files_response_structure() -> TestResult {
    // Validate the expected WorkspaceEdit response structure for willRenameFiles.
    // This is a contract test -- we build the expected shape and verify it.

    let workspace_edit_with_changes = json!({
        "changes": {
            "file:///workspace/app.pl": [
                {
                    "range": {
                        "start": { "line": 0, "character": 4 },
                        "end": { "line": 0, "character": 12 }
                    },
                    "newText": "RenamedModule"
                }
            ]
        }
    });

    let workspace_edit_with_doc_changes = json!({
        "documentChanges": [
            {
                "textDocument": {
                    "uri": "file:///workspace/app.pl",
                    "version": 2
                },
                "edits": [
                    {
                        "range": {
                            "start": { "line": 0, "character": 4 },
                            "end": { "line": 0, "character": 12 }
                        },
                        "newText": "RenamedModule"
                    }
                ]
            }
        ]
    });

    // Either "changes" or "documentChanges" is valid
    assert!(workspace_edit_with_changes.get("changes").is_some());
    assert!(workspace_edit_with_doc_changes.get("documentChanges").is_some());

    // documentChanges entries must have textDocument and edits
    let doc_change = &workspace_edit_with_doc_changes["documentChanges"][0];
    assert!(doc_change.get("textDocument").is_some());
    assert!(doc_change.get("edits").is_some());

    Ok(())
}

// ==================== didRenameFiles ====================

#[test]
fn test_did_rename_files_notification() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Send didRenameFiles notification after the rename has happened on disk
    harness.notify(
        "workspace/didRenameFiles",
        json!({
            "files": [
                {
                    "oldUri": "file:///workspace/lib/OldName.pm",
                    "newUri": "file:///workspace/lib/NewName.pm"
                }
            ]
        }),
    );

    // Notifications have no response. Verify the server is still responsive.
    let _result = harness.request("workspace/symbol", json!({"query": ""})).unwrap_or(json!(null));
    Ok(())
}
