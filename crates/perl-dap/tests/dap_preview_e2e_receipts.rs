//! Deterministic E2E receipts for preview DAP features.
//!
//! These tests validate preview behavior without relying on external debugger timing.

use perl_dap::breakpoints::BreakpointStore;
use perl_dap::protocol::{SetBreakpointsArguments, Source, SourceBreakpoint};
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::{Value, json};
use std::fs::write;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn expect_success(response: DapMessage, command: &str) -> Result<Option<Value>, String> {
    match response {
        DapMessage::Response { success, command: actual, body, message, .. } => {
            if actual != command {
                return Err(format!("expected `{command}` response, got `{actual}`"));
            }
            if !success {
                return Err(format!(
                    "command `{command}` failed: {}",
                    message.unwrap_or_else(|| "<no message>".to_string())
                ));
            }
            Ok(body)
        }
        _ => Err(format!("expected response message for `{command}`")),
    }
}

#[test]
fn preview_hit_condition_receipt() -> TestResult {
    let workspace = tempdir()?;
    let script = workspace.path().join("hit_condition_receipt.pl");
    write(
        &script,
        r#"use strict;
use warnings;
my $x = 0;
$x++;
$x++;
"#,
    )?;
    let script_path = script.to_string_lossy().to_string();

    let store = BreakpointStore::new();
    let response = store.set_breakpoints(&SetBreakpointsArguments {
        source: Source {
            path: Some(script_path.clone()),
            name: Some("hit_condition_receipt.pl".to_string()),
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 4,
            column: None,
            condition: None,
            hit_condition: Some("=2".to_string()),
            log_message: None,
        }]),
        source_modified: None,
    });

    assert_eq!(response.len(), 1);
    assert!(response[0].verified, "hitCondition breakpoint should verify on executable line");

    let first = store.register_breakpoint_hit(&script_path, 4);
    assert!(first.matched, "first hit should match breakpoint");
    assert!(!first.should_stop, "first hit should not stop for hitCondition `=2`");
    assert!(first.log_messages.is_empty(), "first hit should not emit logpoint output");

    let second = store.register_breakpoint_hit(&script_path, 4);
    assert!(second.matched, "second hit should match breakpoint");
    assert!(second.should_stop, "second hit should stop for hitCondition `=2`");
    assert!(second.log_messages.is_empty(), "hitCondition-only breakpoint should not emit logs");

    Ok(())
}

#[test]
fn preview_log_message_receipt() -> TestResult {
    let workspace = tempdir()?;
    let script = workspace.path().join("log_message_receipt.pl");
    write(
        &script,
        r#"use strict;
use warnings;
my $x = 0;
$x++;
$x++;
"#,
    )?;
    let script_path = script.to_string_lossy().to_string();

    let store = BreakpointStore::new();
    let response = store.set_breakpoints(&SetBreakpointsArguments {
        source: Source {
            path: Some(script_path.clone()),
            name: Some("log_message_receipt.pl".to_string()),
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 4,
            column: None,
            condition: None,
            hit_condition: Some("%2".to_string()),
            log_message: Some("loop tick".to_string()),
        }]),
        source_modified: None,
    });

    assert_eq!(response.len(), 1);
    assert!(response[0].verified, "logpoint should verify on executable line");

    let first = store.register_breakpoint_hit(&script_path, 4);
    assert!(first.matched, "first hit should match logpoint");
    assert!(!first.should_stop, "logpoint should not stop execution");
    assert!(first.log_messages.is_empty(), "logpoint with `%2` should not emit on first hit");

    let second = store.register_breakpoint_hit(&script_path, 4);
    assert!(second.matched, "second hit should match logpoint");
    assert!(!second.should_stop, "logpoint should continue execution on second hit");
    assert_eq!(second.log_messages, vec!["loop tick".to_string()]);

    Ok(())
}

#[test]
fn preview_set_exception_breakpoints_receipt() -> TestResult {
    let mut adapter = DebugAdapter::new();

    let init = expect_success(adapter.handle_request(1, "initialize", None), "initialize")?
        .ok_or("initialize response missing body")?;
    let supports_exception_options =
        init.get("supportsExceptionOptions").and_then(Value::as_bool).unwrap_or(false);
    let supports_exception_filter_options =
        init.get("supportsExceptionFilterOptions").and_then(Value::as_bool).unwrap_or(false);
    let filters = init
        .get("exceptionBreakpointFilters")
        .and_then(Value::as_array)
        .ok_or("initialize response missing exceptionBreakpointFilters array")?;

    if supports_exception_options || supports_exception_filter_options {
        assert!(
            !filters.is_empty(),
            "exceptionBreakpointFilters should be non-empty when exception support is advertised"
        );
    }

    let _enable_by_filters = expect_success(
        adapter.handle_request(
            2,
            "setExceptionBreakpoints",
            Some(json!({
                "filters": ["die"]
            })),
        ),
        "setExceptionBreakpoints",
    )?;

    let _enable_by_filter_options = expect_success(
        adapter.handle_request(
            3,
            "setExceptionBreakpoints",
            Some(json!({
                "filterOptions": [{"filterId": "die"}]
            })),
        ),
        "setExceptionBreakpoints",
    )?;

    let _disable = expect_success(
        adapter.handle_request(
            4,
            "setExceptionBreakpoints",
            Some(json!({
                "filters": []
            })),
        ),
        "setExceptionBreakpoints",
    )?;

    Ok(())
}
