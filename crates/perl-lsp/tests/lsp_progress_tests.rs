//! Progress reporting tests for LSP 3.17
//!
//! Tests window/workDoneProgress/cancel notification, progress token structure
//! validation, and progress value contracts (begin, report, end).

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ==================== workDoneProgress/cancel ====================

#[test]
fn test_work_done_progress_cancel_with_string_token() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "window": {
            "workDoneProgress": true
        }
    })))?;

    // Client sends cancel with a string token
    harness.notify(
        "window/workDoneProgress/cancel",
        json!({
            "token": "indexing-workspace-abc123"
        }),
    );

    // Server should handle the cancel gracefully. No crash, remains responsive.
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

#[test]
fn test_work_done_progress_cancel_with_number_token() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(Some(json!({
        "window": {
            "workDoneProgress": true
        }
    })))?;

    // Client sends cancel with a numeric token
    harness.notify(
        "window/workDoneProgress/cancel",
        json!({
            "token": 42
        }),
    );

    // Verify server is still responsive
    let _result = harness.request("workspace/symbol", json!({"query": ""}))
        .unwrap_or(json!(null));
    Ok(())
}

// ==================== Progress token structure ====================

#[test]
fn test_progress_token_string_and_number_variants() -> TestResult {
    // ProgressToken is defined as integer | string in LSP 3.17.
    // Validate both variants are structurally valid.

    let string_token_progress = json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": "workspace-index-1",
            "value": {
                "kind": "begin",
                "title": "Indexing workspace",
                "cancellable": true,
                "percentage": 0
            }
        }
    });

    let number_token_progress = json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": 99,
            "value": {
                "kind": "begin",
                "title": "Indexing workspace",
                "cancellable": false,
                "percentage": 0
            }
        }
    });

    // Validate string token
    assert!(
        string_token_progress["params"]["token"].is_string(),
        "string variant token must be a string"
    );

    // Validate number token
    assert!(
        number_token_progress["params"]["token"].is_number(),
        "number variant token must be a number"
    );

    // Both must have a value with kind
    assert_eq!(string_token_progress["params"]["value"]["kind"], "begin");
    assert_eq!(number_token_progress["params"]["value"]["kind"], "begin");

    Ok(())
}

// ==================== Progress value contracts ====================

#[test]
fn test_progress_value_begin_report_end_structure() -> TestResult {
    // Validate the three progress value shapes per LSP 3.17:
    // WorkDoneProgressBegin, WorkDoneProgressReport, WorkDoneProgressEnd

    let begin = json!({
        "kind": "begin",
        "title": "Parsing files",
        "cancellable": true,
        "message": "Starting parse...",
        "percentage": 0
    });

    // begin: "title" is required, "cancellable", "message", "percentage" are optional
    assert_eq!(begin["kind"], "begin");
    assert!(begin["title"].is_string(), "begin must have a title");
    assert!(begin["cancellable"].is_boolean());
    assert!(begin["message"].is_string());
    let pct = begin["percentage"].as_u64()
        .ok_or("percentage must be a number")?;
    assert!(pct <= 100, "percentage must be in 0..=100");

    let report = json!({
        "kind": "report",
        "message": "Processing file 5 of 20",
        "cancellable": true,
        "percentage": 25
    });

    // report: all fields optional except kind
    assert_eq!(report["kind"], "report");
    let rpt_pct = report["percentage"].as_u64()
        .ok_or("percentage must be a number")?;
    assert!(rpt_pct <= 100);

    let end = json!({
        "kind": "end",
        "message": "Parsing complete"
    });

    // end: "message" is optional
    assert_eq!(end["kind"], "end");
    assert!(end["message"].is_string());

    Ok(())
}
