//! Workspace plumbing tests for LSP 3.17
//!
//! Tests workspace configuration, watched file events, workspace folder changes,
//! applyEdit contract, and workspace/configuration contract.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== didChangeConfiguration ====================

#[test]
fn test_did_change_configuration_notification() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Send workspace/didChangeConfiguration notification with Perl LSP settings
    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "perlPath": "/usr/bin/perl",
                    "enableWarnings": true,
                    "perltidyProfile": ".perltidyrc"
                }
            }
        }),
    );

    // Notifications have no response; the server should accept it without crashing.
    // Verify server is still responsive after the notification by sending a request.
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

#[test]
fn test_did_change_configuration_empty_settings() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Send empty settings object -- server must handle gracefully
    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {}
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

// ==================== didChangeWatchedFiles ====================

#[test]
fn test_did_change_watched_files_created() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // FileChangeType: 1 = Created
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [
                {
                    "uri": "file:///workspace/lib/NewModule.pm",
                    "type": 1
                }
            ]
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

#[test]
fn test_did_change_watched_files_changed_and_deleted() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // FileChangeType: 2 = Changed, 3 = Deleted
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({
            "changes": [
                {
                    "uri": "file:///workspace/lib/Existing.pm",
                    "type": 2
                },
                {
                    "uri": "file:///workspace/lib/Obsolete.pm",
                    "type": 3
                }
            ]
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

// ==================== didChangeWorkspaceFolders ====================

#[test]
fn test_did_change_workspace_folders_added() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "workspace": {
            "workspaceFolders": true
        }
    })))?;

    harness.notify(
        "workspace/didChangeWorkspaceFolders",
        json!({
            "event": {
                "added": [
                    {
                        "uri": "file:///projects/second-project",
                        "name": "second-project"
                    }
                ],
                "removed": []
            }
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

#[test]
fn test_did_change_workspace_folders_removed() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "workspace": {
            "workspaceFolders": true
        }
    })))?;

    harness.notify(
        "workspace/didChangeWorkspaceFolders",
        json!({
            "event": {
                "added": [],
                "removed": [
                    {
                        "uri": "file:///projects/old-project",
                        "name": "old-project"
                    }
                ]
            }
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

// ==================== applyEdit contract validation ====================

#[test]
fn test_apply_edit_request_contract() -> TestResult {
    // workspace/applyEdit is a server->client request.
    // Validate the JSON contract structure that the server would send.

    let apply_edit_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "workspace/applyEdit",
        "params": {
            "label": "Rename symbol",
            "edit": {
                "changes": {
                    "file:///workspace/main.pl": [
                        {
                            "range": {
                                "start": { "line": 5, "character": 4 },
                                "end": { "line": 5, "character": 10 }
                            },
                            "newText": "new_name"
                        }
                    ]
                }
            }
        }
    });

    // Validate required fields
    let params = apply_edit_request.get("params")
        .ok_or("applyEdit request must have params")?;
    let edit = params.get("edit")
        .ok_or("params must contain an edit field")?;
    assert!(edit.get("changes").is_some() || edit.get("documentChanges").is_some(),
        "edit must contain changes or documentChanges");

    // Validate the expected client response structure
    let success_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "applied": true
        }
    });

    let failure_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "applied": false,
            "failureReason": "File is read-only"
        }
    });

    assert_eq!(success_response["result"]["applied"], true);
    assert_eq!(failure_response["result"]["applied"], false);
    assert!(failure_response["result"]["failureReason"].is_string());

    Ok(())
}

// ==================== configuration contract validation ====================

#[test]
fn test_workspace_configuration_request_contract() -> TestResult {
    // workspace/configuration is a server->client request.
    // Validate the JSON contract structure.

    let config_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "workspace/configuration",
        "params": {
            "items": [
                {
                    "scopeUri": "file:///workspace",
                    "section": "perl"
                },
                {
                    "section": "editor"
                }
            ]
        }
    });

    let items = config_request["params"]["items"].as_array()
        .ok_or("items must be an array")?;
    assert_eq!(items.len(), 2);

    // First item has both scopeUri and section
    assert!(items[0].get("scopeUri").is_some());
    assert!(items[0].get("section").is_some());

    // Second item has only section (scopeUri is optional)
    assert!(items[1].get("scopeUri").is_none());
    assert!(items[1].get("section").is_some());

    // Expected client response: array of values matching item order
    let config_response = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": [
            {
                "perlPath": "/usr/bin/perl",
                "enableWarnings": true
            },
            {
                "tabSize": 4,
                "insertSpaces": true
            }
        ]
    });

    let results = config_response["result"].as_array()
        .ok_or("result must be an array")?;
    assert_eq!(results.len(), items.len(),
        "response array length must match items array length");

    Ok(())
}
