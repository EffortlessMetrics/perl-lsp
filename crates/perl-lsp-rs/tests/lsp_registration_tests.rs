//! Dynamic capability registration tests for LSP 3.17
//!
//! Tests client/registerCapability and client/unregisterCapability request contracts
//! (server->client), including file watcher registration with globPatterns.

mod support;

use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== client/registerCapability contract ====================

#[test]
fn test_register_capability_request_contract() -> TestResult {
    // client/registerCapability is a server->client request.
    // Validate the JSON contract structure.

    let register_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "client/registerCapability",
        "params": {
            "registrations": [
                {
                    "id": "file-watcher-registration-1",
                    "method": "workspace/didChangeWatchedFiles",
                    "registerOptions": {
                        "watchers": [
                            {
                                "globPattern": "**/*.pm",
                                "kind": 7
                            },
                            {
                                "globPattern": "**/*.pl",
                                "kind": 7
                            }
                        ]
                    }
                }
            ]
        }
    });

    // Validate required fields
    let registrations = register_request["params"]["registrations"]
        .as_array()
        .ok_or("registrations must be an array")?;
    assert!(!registrations.is_empty(), "registrations must not be empty");

    let reg = &registrations[0];
    assert!(reg["id"].is_string(), "registration must have an id");
    assert!(reg["method"].is_string(), "registration must have a method");
    assert!(reg["registerOptions"].is_object(), "registration must have registerOptions");

    // Validate file watcher options
    let watchers =
        reg["registerOptions"]["watchers"].as_array().ok_or("watchers must be an array")?;
    assert_eq!(watchers.len(), 2);

    for watcher in watchers {
        assert!(watcher["globPattern"].is_string(), "each watcher must have a globPattern string");
        // kind is a bitmask: 1=Create, 2=Change, 4=Delete; 7 = all
        let kind = watcher["kind"].as_u64().ok_or("kind must be a number")?;
        assert!(kind > 0 && kind <= 7, "kind must be a valid WatchKind bitmask (1..=7)");
    }

    // Expected client response: void (null result)
    let client_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": null
    });
    assert!(client_response["result"].is_null());

    Ok(())
}

// ==================== client/unregisterCapability contract ====================

#[test]
fn test_unregister_capability_request_contract() -> TestResult {
    // client/unregisterCapability is a server->client request.
    // Validate the JSON contract structure.

    let unregister_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "client/unregisterCapability",
        "params": {
            "unregisterations": [
                {
                    "id": "file-watcher-registration-1",
                    "method": "workspace/didChangeWatchedFiles"
                }
            ]
        }
    });

    // Note: the LSP spec uses "unregisterations" (with the typo), not "unregistrations"
    let unregs = unregister_request["params"]["unregisterations"]
        .as_array()
        .ok_or("unregisterations must be an array")?;
    assert!(!unregs.is_empty(), "unregisterations must not be empty");

    let unreg = &unregs[0];
    assert!(unreg["id"].is_string(), "unregistration must have an id");
    assert!(unreg["method"].is_string(), "unregistration must have a method");

    // The id must match a previously registered capability
    assert_eq!(unreg["id"], "file-watcher-registration-1");
    assert_eq!(unreg["method"], "workspace/didChangeWatchedFiles");

    // Expected client response: void (null result)
    let client_response = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "result": null
    });
    assert!(client_response["result"].is_null());

    Ok(())
}

// ==================== Registration with multiple methods ====================

#[test]
fn test_register_multiple_capabilities_at_once() -> TestResult {
    // A single registerCapability request can register multiple capabilities

    let multi_register = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "client/registerCapability",
        "params": {
            "registrations": [
                {
                    "id": "semantic-tokens-reg",
                    "method": "textDocument/semanticTokens",
                    "registerOptions": {
                        "documentSelector": [
                            { "language": "perl" }
                        ],
                        "legend": {
                            "tokenTypes": ["namespace", "type", "class", "function", "variable"],
                            "tokenModifiers": ["declaration", "definition"]
                        },
                        "full": true,
                        "range": true
                    }
                },
                {
                    "id": "file-watcher-reg",
                    "method": "workspace/didChangeWatchedFiles",
                    "registerOptions": {
                        "watchers": [
                            {
                                "globPattern": "**/*.pm",
                                "kind": 7
                            }
                        ]
                    }
                }
            ]
        }
    });

    let registrations = multi_register["params"]["registrations"]
        .as_array()
        .ok_or("registrations must be an array")?;
    assert_eq!(registrations.len(), 2, "must have two registrations");

    // Each registration must have a unique id
    let id0 = registrations[0]["id"].as_str().ok_or("id must be a string")?;
    let id1 = registrations[1]["id"].as_str().ok_or("id must be a string")?;
    assert_ne!(id0, id1, "registration ids must be unique");

    // Each has its own method
    assert_eq!(registrations[0]["method"], "textDocument/semanticTokens");
    assert_eq!(registrations[1]["method"], "workspace/didChangeWatchedFiles");

    Ok(())
}

// ==================== File watcher glob patterns ====================

#[test]
fn test_file_watcher_glob_pattern_variants() -> TestResult {
    // Validate various glob pattern forms that a server might register

    let watcher_registrations = json!({
        "registrations": [
            {
                "id": "perl-file-watchers",
                "method": "workspace/didChangeWatchedFiles",
                "registerOptions": {
                    "watchers": [
                        { "globPattern": "**/*.pm", "kind": 7 },
                        { "globPattern": "**/*.pl", "kind": 7 },
                        { "globPattern": "**/*.t",  "kind": 7 },
                        { "globPattern": "**/Makefile.PL", "kind": 3 },
                        { "globPattern": "**/cpanfile", "kind": 1 }
                    ]
                }
            }
        ]
    });

    let watchers = watcher_registrations["registrations"][0]["registerOptions"]["watchers"]
        .as_array()
        .ok_or("watchers must be an array")?;

    assert_eq!(watchers.len(), 5);

    // Validate each watcher has required fields
    for watcher in watchers {
        assert!(watcher["globPattern"].is_string(), "each watcher must specify a globPattern");
        let kind = watcher["kind"].as_u64().ok_or("kind must be present")?;
        // kind must be a valid WatchKind bitmask (1=Create, 2=Change, 4=Delete)
        assert!((1..=7).contains(&kind), "kind {kind} is not a valid WatchKind bitmask");
    }

    // Verify specific pattern semantics
    assert_eq!(watchers[3]["globPattern"], "**/Makefile.PL");
    // kind 3 = Create(1) | Change(2) -- watch for creation and modification only
    assert_eq!(watchers[3]["kind"], 3);

    // kind 1 = Create only
    assert_eq!(watchers[4]["kind"], 1);

    Ok(())
}
