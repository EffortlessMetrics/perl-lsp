use perl_dap::{DapMessage, DebugAdapter};
use perl_tdd_support::must;
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper: send a completions request and return the response body.
fn completions_request(
    adapter: &mut DebugAdapter,
    text: &str,
    column: i64,
) -> Result<serde_json::Value, String> {
    let response =
        adapter.handle_request(2, "completions", Some(json!({ "text": text, "column": column })));
    match response {
        DapMessage::Response { success: true, body: Some(body), .. } => Ok(body),
        DapMessage::Response { success: false, message, .. } => {
            Err(message.unwrap_or_else(|| "unknown error".to_string()))
        }
        _ => Err("unexpected response type".to_string()),
    }
}

fn extract_labels(body: &serde_json::Value) -> Vec<String> {
    body.get("targets")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("label").and_then(|l| l.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn test_completions_basic_keyword_match() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = completions_request(&mut adapter, "pr", 2).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    assert!(labels.contains(&"print".to_string()), "Expected 'print' in {labels:?}");
    assert!(labels.contains(&"printf".to_string()), "Expected 'printf' in {labels:?}");

    // All results should start with "pr"
    for label in &labels {
        assert!(label.starts_with("pr"), "Unexpected match: {label}");
    }

    Ok(())
}

#[test]
fn test_completions_empty_prefix() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = completions_request(&mut adapter, "", 0).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    // Empty prefix should return all keywords
    assert!(labels.len() > 50, "Expected many keywords, got {}", labels.len());
    assert!(labels.contains(&"my".to_string()));
    assert!(labels.contains(&"sub".to_string()));
    assert!(labels.contains(&"use".to_string()));

    Ok(())
}

#[test]
fn test_completions_no_match() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = completions_request(&mut adapter, "zzz", 3).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    assert!(labels.is_empty(), "Expected no matches for 'zzz', got {labels:?}");

    Ok(())
}

#[test]
fn test_completions_missing_arguments() {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let response = adapter.handle_request(2, "completions", None);

    match response {
        DapMessage::Response { success, message, .. } => {
            assert!(!success, "Expected failure for missing arguments");
            assert!(message.is_some(), "Expected error message");
        }
        _ => must(Err::<(), _>("Expected response message")),
    }
}

#[test]
fn test_completions_mid_word() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    // Typing "my $x = pri" with column at 11 should extract stem "pri"
    let body = completions_request(&mut adapter, "my $x = pri", 11).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    assert!(labels.contains(&"print".to_string()), "Expected 'print' in {labels:?}");
    assert!(labels.contains(&"printf".to_string()), "Expected 'printf' in {labels:?}");

    Ok(())
}

#[test]
fn test_completions_column_zero() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    // Column 0 means empty prefix
    let body = completions_request(&mut adapter, "print", 0).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    // Should return all keywords since prefix is ""
    assert!(labels.len() > 50, "Column 0 should return all keywords");

    Ok(())
}

#[test]
fn test_completions_column_equals_length() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let text = "print";
    let body =
        completions_request(&mut adapter, text, text.len() as i64).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    assert!(labels.contains(&"print".to_string()), "Expected exact match 'print' in {labels:?}");
    assert!(labels.contains(&"printf".to_string()), "Expected 'printf' in {labels:?}");

    Ok(())
}

#[test]
fn test_completions_column_beyond_length() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    // Column beyond text length should be clamped safely
    let body = completions_request(&mut adapter, "pr", 100).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    // Should still work (clamped to text length)
    assert!(labels.contains(&"print".to_string()), "Expected 'print' in {labels:?}");

    Ok(())
}

#[test]
fn test_completions_underscore_prefix() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = completions_request(&mut adapter, "un", 2).map_err(|e| e.to_string())?;
    let labels = extract_labels(&body);

    assert!(labels.contains(&"unless".to_string()), "Expected 'unless' in {labels:?}");
    assert!(labels.contains(&"unshift".to_string()), "Expected 'unshift' in {labels:?}");
    assert!(labels.contains(&"untie".to_string()), "Expected 'untie' in {labels:?}");
    assert!(labels.contains(&"unpack".to_string()), "Expected 'unpack' in {labels:?}");

    Ok(())
}

#[test]
fn test_completions_all_targets_have_keyword_type() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let body = completions_request(&mut adapter, "s", 1).map_err(|e| e.to_string())?;
    let targets = body.get("targets").and_then(|v| v.as_array()).ok_or("missing targets")?;

    for target in targets {
        let type_val = target.get("type").and_then(|v| v.as_str());
        assert!(
            type_val.is_some_and(|t| t == "keyword"
                || t == "variable"
                || t == "module"
                || t == "function"),
            "Expected a valid completion type, got {type_val:?}"
        );
    }

    Ok(())
}

#[test]
fn test_completions_response_is_success() {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);

    let response =
        adapter.handle_request(2, "completions", Some(json!({ "text": "my", "column": 2 })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success);
            assert_eq!(command, "completions");
        }
        _ => must(Err::<(), _>("Expected response message")),
    }
}

mod proptest_completions {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_completions_arbitrary_input(
            text in "\\PC{0,200}",
            column in -10i64..300
        ) {
            let mut adapter = DebugAdapter::new();
            adapter.handle_request(1, "initialize", None);

            // Must never panic regardless of input
            let response = adapter.handle_request(
                2,
                "completions",
                Some(json!({ "text": text, "column": column })),
            );

            // Should always be a response
            match response {
                DapMessage::Response { command, .. } => {
                    assert_eq!(command, "completions");
                }
                _ => must(Err::<(), _>("Expected response")),
            }
        }
    }
}
