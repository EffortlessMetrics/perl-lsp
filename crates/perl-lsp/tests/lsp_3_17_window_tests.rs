//! LSP 3.17 Window and Miscellaneous Contract Tests
//!
//! Tests for window/showMessage, window/showDocument, window/logMessage,
//! window/workDoneProgress, $/cancelRequest, $/progress, $/setTrace,
//! $/logTrace, and telemetry/event.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== WINDOW FEATURES ====================

#[test]
fn test_show_message_3_17() -> TestResult {
    // Server->client, so we can't test directly
    // But we document the contract here

    // window/showMessage notification
    // { "type": 1-4, "message": "string" }

    // window/showMessageRequest
    // { "type": 1-4, "message": "string", "actions": [{"title": "string"}] }
    // Response: MessageActionItem | null
    Ok(())
}

#[test]
fn test_show_document_3_17() -> TestResult {
    // Server->client request (3.16+)
    // Params: { uri, external?, takeFocus?, selection? }
    // Response: { success: boolean }
    Ok(())
}

#[test]
fn test_log_message_3_17() -> TestResult {
    // Server->client notification
    // { "type": 1-4, "message": "string" }
    Ok(())
}

#[test]
fn test_work_done_progress_3_17() -> TestResult {
    // window/workDoneProgress/create (server->client)
    // $/progress notifications
    // window/workDoneProgress/cancel (client->server)
    Ok(())
}

// ==================== MISCELLANEOUS ====================

#[test]
fn test_cancel_request_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Send a request then immediately cancel it
    // In real scenario, this would be async
    harness.notify(
        "$/cancelRequest",
        json!({
            "id": 999
        }),
    );
    Ok(())
}

#[test]
fn test_progress_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    // Progress notifications can flow either way
    harness.notify(
        "$/progress",
        json!({
            "token": "test-progress",
            "value": {
                "kind": "begin",
                "title": "Processing",
                "cancellable": true,
                "percentage": 0
            }
        }),
    );

    harness.notify(
        "$/progress",
        json!({
            "token": "test-progress",
            "value": {
                "kind": "report",
                "message": "Working...",
                "percentage": 50
            }
        }),
    );

    harness.notify(
        "$/progress",
        json!({
            "token": "test-progress",
            "value": {
                "kind": "end",
                "message": "Complete"
            }
        }),
    );
    Ok(())
}

#[test]
fn test_set_trace_3_17() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    harness.notify(
        "$/setTrace",
        json!({
            "value": "verbose"  // off | messages | verbose
        }),
    );
    Ok(())
}

#[test]
fn test_log_trace_3_17() -> TestResult {
    // Server->client notification when tracing is on
    // { "message": "string", "verbose"?: "string" }
    Ok(())
}

#[test]
fn test_telemetry_3_17() -> TestResult {
    // Server->client notification
    // params: any
    Ok(())
}
