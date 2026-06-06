//! Dynamic capability registration tests for LSP 3.17
//!
//! Tests client/registerCapability and client/unregisterCapability request contracts
//! (server->client), including file watcher registration with globPatterns.

mod support;

use perl_lsp_rs_core::runtime::tuning::RuntimeTuning;
use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

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

#[test]
fn e2e_mode_does_not_register_file_watchers() -> TestResult {
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::e2e_defaults());
    harness.initialize(Some(watcher_dynamic_capabilities()))?;

    let requests = harness.drain_server_requests(250);
    assert!(
        !has_registration(&requests, "workspace/didChangeWatchedFiles"),
        "e2e runtime tuning must not register file watchers; got {requests:?}"
    );
    Ok(())
}

#[test]
fn normal_mode_registers_file_watchers_when_client_supports_dynamic_watchers() -> TestResult {
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::normal_defaults());
    harness.initialize(Some(watcher_dynamic_capabilities()))?;

    let requests = harness.drain_server_requests(250);
    let registration = registration_for_method(&requests, "workspace/didChangeWatchedFiles")
        .ok_or("normal mode must register file watchers when client supports them")?;
    let watchers = registration
        .pointer("/registerOptions/watchers")
        .and_then(Value::as_array)
        .ok_or("file watcher registration must include watchers")?;

    assert!(watchers.iter().any(|watcher| watcher["globPattern"] == json!("**/*.pl")));
    assert!(watchers.iter().any(|watcher| watcher["globPattern"] == json!("**/*.pm")));
    Ok(())
}

#[test]
fn relative_pattern_clients_receive_relative_file_watchers() -> TestResult {
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::normal_defaults());
    harness
        .initialize_with_root("file:///workspace", Some(watcher_relative_pattern_capabilities()))?;

    let requests = harness.drain_server_requests(250);
    let registration = registration_for_method(&requests, "workspace/didChangeWatchedFiles")
        .ok_or("normal mode must register file watchers when client supports them")?;
    let watchers = registration
        .pointer("/registerOptions/watchers")
        .and_then(Value::as_array)
        .ok_or("file watcher registration must include watchers")?;

    assert_eq!(
        watchers.len(),
        4,
        "single-root registration should include one watcher per Perl pattern"
    );
    for watcher in watchers {
        assert_eq!(watcher.get("kind").and_then(Value::as_u64), Some(7));
        assert_eq!(
            watcher.pointer("/globPattern/baseUri").and_then(Value::as_str),
            Some("file:///workspace"),
            "relative watcher must be rooted at the initialized workspace: {watcher}"
        );
        assert!(
            watcher.pointer("/globPattern/pattern").and_then(Value::as_str).is_some(),
            "relative watcher must carry a pattern string: {watcher}"
        );
    }
    assert!(
        watchers.iter().any(|watcher| {
            watcher.pointer("/globPattern/pattern").and_then(Value::as_str) == Some("**/*.pl")
        }),
        "expected .pl watcher in relative-pattern registration: {watchers:?}"
    );
    assert!(
        watchers.iter().any(|watcher| {
            watcher.pointer("/globPattern/pattern").and_then(Value::as_str) == Some("**/*.pm")
        }),
        "expected .pm watcher in relative-pattern registration: {watchers:?}"
    );
    Ok(())
}

#[test]
fn relative_pattern_clients_fall_back_to_string_watchers_without_valid_workspace_uri() -> TestResult
{
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::normal_defaults());
    harness.initialize_with_root("not a uri", Some(watcher_relative_pattern_capabilities()))?;

    let requests = harness.drain_server_requests(250);
    let registration = registration_for_method(&requests, "workspace/didChangeWatchedFiles")
        .ok_or("normal mode must register fallback file watchers when workspace URI is invalid")?;
    let watchers = registration
        .pointer("/registerOptions/watchers")
        .and_then(Value::as_array)
        .ok_or("file watcher registration must include watchers")?;

    assert!(watchers.iter().any(|watcher| watcher["globPattern"] == json!("**/*.pl")));
    assert!(watchers.iter().any(|watcher| watcher["globPattern"] == json!("**/*.pm")));
    for watcher in watchers {
        assert!(
            watcher.get("globPattern").is_some_and(Value::is_string),
            "invalid workspace URI must fall back to string glob patterns: {watcher}"
        );
    }
    Ok(())
}

#[test]
fn jetbrains_still_disables_file_watchers_even_if_client_advertises_support() -> TestResult {
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::normal_defaults());
    let init = harness.request_raw(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "clientInfo": { "name": "JetBrains" },
            "capabilities": watcher_dynamic_capabilities(),
            "rootUri": "file:///workspace"
        }
    }));
    assert!(
        init.pointer("/result/capabilities").is_some(),
        "initialize must succeed for JetBrains-shaped client, got {init:?}"
    );

    harness.notify("initialized", json!({}));
    let requests = harness.drain_server_requests(250);
    assert!(
        !has_registration(&requests, "workspace/didChangeWatchedFiles"),
        "JetBrains watcher suppression must remain separate from runtime tuning; got {requests:?}"
    );
    Ok(())
}

#[test]
fn inline_completion_dynamic_registration_not_blocked_by_file_watchers_false() -> TestResult {
    let mut harness = LspHarness::new_with_tuning(RuntimeTuning::e2e_defaults());
    harness.initialize(Some(json!({
        "workspace": {
            "didChangeWatchedFiles": {
                "dynamicRegistration": true
            }
        },
        "textDocument": {
            "inlineCompletion": {
                "dynamicRegistration": true
            }
        }
    })))?;

    let requests = harness.drain_server_requests(250);
    assert!(
        !has_registration(&requests, "workspace/didChangeWatchedFiles"),
        "file watcher registration must remain disabled when runtime tuning disables watchers"
    );
    assert!(
        has_registration(&requests, "textDocument/inlineCompletion"),
        "inline completion dynamic registration must not be gated by file watcher tuning; got {requests:?}"
    );
    Ok(())
}

#[test]
fn guardrail_inline_completion_registration_and_capability_wiring() -> TestResult {
    let lifecycle_caps = include_str!("../src/runtime/lifecycle/capabilities.rs");
    assert!(
        !lifecycle_caps.contains(
            "capabilities[\"experimental\"] = json!({\n            \"perlInlineCompletionStream\": true\n        });"
        ),
        "initialize must not overwrite capabilities.experimental with only perlInlineCompletionStream"
    );

    let watchers = include_str!("../src/runtime/lifecycle/watchers.rs");
    assert!(
        !watchers.contains("self.outbound.send_request"),
        "inline-completion dynamic registration must use self.send_request"
    );
    let inline_registration = watchers
        .split("pub(crate) fn register_inline_completion_if_needed")
        .nth(1)
        .ok_or("inline completion registration function must exist")?;
    assert!(
        !inline_registration.contains(".dynamic_registration_support")
            && !inline_registration.contains("caps.dynamic_registration_support")
            && !inline_registration
                .contains("client_capabilities.lock().dynamic_registration_support"),
        "inline-completion dynamic registration must not be gated on watcher dynamic registration"
    );

    Ok(())
}

fn watcher_dynamic_capabilities() -> Value {
    json!({
        "workspace": {
            "didChangeWatchedFiles": {
                "dynamicRegistration": true
            }
        },
        "textDocument": {}
    })
}

fn watcher_relative_pattern_capabilities() -> Value {
    json!({
        "workspace": {
            "didChangeWatchedFiles": {
                "dynamicRegistration": true,
                "relativePatternSupport": true
            }
        },
        "textDocument": {}
    })
}

fn registration_for_method<'a>(requests: &'a [Value], method: &str) -> Option<&'a Value> {
    requests.iter().find_map(|request| {
        if request.get("method").and_then(Value::as_str) != Some("client/registerCapability") {
            return None;
        }

        request.pointer("/params/registrations").and_then(Value::as_array).and_then(
            |registrations| {
                registrations
                    .iter()
                    .find(|entry| entry.get("method").and_then(Value::as_str) == Some(method))
            },
        )
    })
}

fn has_registration(requests: &[Value], method: &str) -> bool {
    registration_for_method(requests, method).is_some()
}
