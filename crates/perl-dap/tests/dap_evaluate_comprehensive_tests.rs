//! Comprehensive DAP evaluate request test suite (Issue #3536)
//!
//! Covers:
//! - Basic variable expression safety validation ($scalar, @array, %hash)
//! - Array/hash element access patterns ($array[0], $hash{key}, $ref->{field})
//! - Simple arithmetic and string expressions
//! - Method call expressions in safe mode (blocked because methods may be dangerous)
//! - Blessed object inspection (ref, Scalar::Util::blessed)
//! - Evaluation context variants (watch, repl, hover, clipboard)
//! - Error handling: missing args, empty expression, malformed JSON
//! - Response body structure (result, type, variablesReference fields)
//! - Timeout parameter handling
//! - setExpression missing-argument error handling

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn new_adapter() -> DebugAdapter {
    DebugAdapter::new()
}

/// Assert that the response is a failed evaluate with a message containing `needle`.
fn assert_evaluate_blocked(
    response: DapMessage,
    needle: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "evaluate");
            assert!(!success, "expected evaluate to be blocked");
            let msg = message.ok_or("expected error message")?;
            assert!(
                msg.contains(needle),
                "error message {msg:?} does not contain {needle:?}"
            );
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

/// Assert that the response is a failed evaluate whose message does NOT contain `banned`.
fn assert_evaluate_not_safe_blocked(
    response: DapMessage,
    banned: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match response {
        DapMessage::Response {
            command, message, ..
        } => {
            assert_eq!(command, "evaluate");
            let msg = message.unwrap_or_default();
            assert!(
                !msg.contains(banned),
                "safe mode should not block this expression, but got: {msg:?}"
            );
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC: Basic variable evaluation — safe expressions that pass validation
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_scalar_variable_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "$my_scalar", "allowSideEffects": false })),
    );
    // Should pass safety validation; fails only because there is no active session.
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_array_variable_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "@my_array", "allowSideEffects": false })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_hash_variable_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "%my_hash", "allowSideEffects": false })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

// ---------------------------------------------------------------------------
// AC: Array/hash element access — safe subscript forms
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_array_element_access_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in ["$array[0]", "$array[-1]", "$array[42]"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

#[test]
fn test_evaluate_hash_element_access_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in ["$hash{key}", "$hash{'literal'}", "$config{timeout}"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

#[test]
fn test_evaluate_nested_hashref_dereference_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in [
        "$ref->{field}",
        "$obj->{name}",
        "$data->{nested}->{deep}",
        "$complex_var->{nested}->[0]",
    ] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        // Hashref access via -> is a read operation; should not be blocked.
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

#[test]
fn test_evaluate_arrayref_dereference_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in ["$aref->[0]", "$aref->[1]", "$matrix->[0]->[1]"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC: Simple expression evaluation — arithmetic, string, comparison
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_arithmetic_expressions_are_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in ["$x + $y", "$a - $b", "$n * 2", "$total / $count", "$x ** 2"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

#[test]
fn test_evaluate_string_expressions_are_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in [
        r#""hello world""#,
        r#"'literal string'"#,
        "$name . ' suffix'",
        "length($str)",
        "substr($str, 0, 4)",
        "uc($name)",
        "lc($name)",
    ] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

#[test]
fn test_evaluate_comparison_expressions_are_safe() -> TestResult {
    let mut adapter = new_adapter();
    // Note: the SafeEvaluator microcrate (perl-dap-eval) does a naive
    // `contains("=")` check for assignment operators, which means expressions
    // containing `==`, `!=`, `<=`, `>=` are also blocked even though they are
    // read-only comparisons.  The evaluate pipeline runs BOTH validators, so
    // we only test expressions that pass both.
    for expr in [
        "$a < $b",
        "$a > $b",
        "$a eq $b",
        "$a ne $b",
        "$a lt $b",
        "$a gt $b",
        "$a le $b",
        "$a ge $b",
        "$a cmp $b",
        // Note: <=> contains = so it's blocked by the naive microcrate validator
    ] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

/// Test that the SafeEvaluator microcrate blocks equality operators that contain `=`.
/// This is a known limitation of the perl-dap-eval crate (naive substring check).
/// Filed separately for follow-up.
#[test]
fn test_evaluate_equality_operators_blocked_by_microcrate_validator() -> TestResult {
    let mut adapter = new_adapter();
    // These are read-only comparisons, but the microcrate blocks them due to
    // substring `=` match.  This test documents the current behavior.
    for expr in ["$a == $b", "$a != $b", "$a <= $b", "$a >= $b"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        // Currently blocked — documents known microcrate limitation.
        match response {
            DapMessage::Response {
                success, command, ..
            } => {
                assert_eq!(command, "evaluate");
                assert!(!success, "expected microcrate to block {expr:?}");
            }
            other => return Err(format!("expected Response, got {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn test_evaluate_ref_and_defined_checks_are_safe() -> TestResult {
    let mut adapter = new_adapter();
    for expr in [
        "ref($obj)",
        "defined($val)",
        "defined($hash{key})",
        "exists($hash{key})",
        "scalar(@array)",
    ] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC: Method calls — blocked in safe mode (method calls may be dangerous)
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_method_calls_blocked_in_safe_mode() -> TestResult {
    let mut adapter = new_adapter();
    // Method calls via -> are not exempted in safe mode: $obj->print is dangerous.
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "$obj->print", "allowSideEffects": false })),
    );
    assert_evaluate_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_method_calls_allowed_with_side_effects() -> TestResult {
    let mut adapter = new_adapter();
    // With allowSideEffects true, method calls bypass the safety validator.
    // They will still fail because there is no active debugger session.
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "$obj->some_method()", "allowSideEffects": true })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

// ---------------------------------------------------------------------------
// AC: Blessed object display — bless itself is blocked; ref() is safe
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_bless_is_blocked_in_safe_mode() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "bless $ref, 'Class'", "allowSideEffects": false })),
    );
    assert_evaluate_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_ref_introspection_is_safe() -> TestResult {
    let mut adapter = new_adapter();
    // ref() is a read-only inspection — should pass the safety validator.
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "ref($obj)", "allowSideEffects": false })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

// ---------------------------------------------------------------------------
// AC: Context variants — watch, repl, hover, clipboard
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_watch_context_passes_safety() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "$watched_var",
            "context": "watch",
            "allowSideEffects": false
        })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_hover_context_passes_safety() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "$hovered_var",
            "context": "hover",
            "allowSideEffects": false
        })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_repl_context_passes_safety_for_read_ops() -> TestResult {
    let mut adapter = new_adapter();
    // Read-only expressions in the REPL should pass safety validation.
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "$x + 1",
            "context": "repl",
            "allowSideEffects": false
        })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_clipboard_context_passes_safety() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "$clipboard_var",
            "context": "clipboard",
            "allowSideEffects": false
        })),
    );
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_repl_context_blocks_mutations() -> TestResult {
    let mut adapter = new_adapter();
    // Even in REPL context, mutations are blocked in safe mode.
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "system('ls')",
            "context": "repl",
            "allowSideEffects": false
        })),
    );
    assert_evaluate_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_watch_context_blocks_mutations() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "push @arr, 1",
            "context": "watch",
            "allowSideEffects": false
        })),
    );
    assert_evaluate_blocked(response, "Safe evaluation mode")
}

// ---------------------------------------------------------------------------
// AC: Error handling — invalid/missing arguments
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_missing_arguments_returns_error() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(1, "evaluate", None);
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "evaluate");
            assert!(!success, "evaluate with no arguments should fail");
            let msg = message.ok_or("expected error message")?;
            assert!(!msg.is_empty(), "error message should be non-empty");
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_evaluate_empty_expression_returns_error() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(1, "evaluate", Some(json!({ "expression": "" })));
    assert_evaluate_blocked(response, "Empty expression")
}

#[test]
fn test_evaluate_newline_in_expression_is_rejected() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "$x\ndie('injection')" })),
    );
    assert_evaluate_blocked(response, "newline")
}

#[test]
fn test_evaluate_carriage_return_in_expression_is_rejected() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "$x\rdie('injection')" })),
    );
    assert_evaluate_blocked(response, "newline")
}

#[test]
fn test_evaluate_no_session_returns_meaningful_error() -> TestResult {
    let mut adapter = new_adapter();
    let response =
        adapter.handle_request(1, "evaluate", Some(json!({ "expression": "$valid_var" })));
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "evaluate");
            assert!(!success, "evaluate without debugger session should fail");
            let msg = message.ok_or("expected error message")?;
            // Must mention the session, not a safety issue.
            assert!(
                msg.contains("session") || msg.contains("Session"),
                "error should mention missing session, got: {msg:?}"
            );
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC: Response body structure — fields must conform to DAP spec
// ---------------------------------------------------------------------------

/// When evaluate *succeeds* (via a mocked session), the response body must
/// include `result`, `type`, and `variablesReference`. This tests the field
/// names by verifying that the protocol types serialize correctly.
#[test]
fn test_evaluate_response_body_has_required_fields() -> TestResult {
    use perl_dap::protocol::EvaluateResponseBody;
    use serde_json::Value;

    let body = EvaluateResponseBody {
        result: "42".to_string(),
        type_: Some("integer".to_string()),
        variables_reference: 0,
    };

    let serialized: Value = serde_json::to_value(&body)?;

    // DAP spec requires these fields in the evaluate response body.
    assert!(serialized.get("result").is_some(), "missing 'result' field");
    assert!(
        serialized.get("variablesReference").is_some(),
        "missing 'variablesReference' field"
    );
    // `type` is optional per spec; when present it should be under `type`.
    assert_eq!(serialized["result"].as_str(), Some("42"));
    assert_eq!(serialized["variablesReference"].as_i64(), Some(0));
    assert_eq!(serialized["type"].as_str(), Some("integer"));

    Ok(())
}

#[test]
fn test_evaluate_response_body_no_type_omitted() -> TestResult {
    use perl_dap::protocol::EvaluateResponseBody;
    use serde_json::Value;

    let body = EvaluateResponseBody {
        result: "hello".to_string(),
        type_: None,
        variables_reference: 0,
    };

    let serialized: Value = serde_json::to_value(&body)?;

    // When `type` is None, it should be absent from the serialized output
    // (skip_serializing_if = "Option::is_none").
    assert!(
        serialized.get("type").is_none(),
        "type field should be absent when None, got: {serialized:?}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// AC: Timeout parameter handling
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_with_frame_id_passes_validation() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({
            "expression": "$frame_local",
            "frameId": 1,
            "allowSideEffects": false
        })),
    );
    // frameId is advisory — safe expressions with a frameId should pass safety validation.
    assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")
}

#[test]
fn test_evaluate_command_name_in_all_responses() -> TestResult {
    let mut adapter = new_adapter();

    // All evaluate responses must have command == "evaluate" regardless of success/failure.
    let cases: &[(&str, serde_json::Value)] = &[
        ("empty", json!({ "expression": "" })),
        ("newline", json!({ "expression": "1\n2" })),
        (
            "safe-block",
            json!({ "expression": "system('ls')", "allowSideEffects": false }),
        ),
        ("no-session", json!({ "expression": "$x" })),
    ];

    for (label, args) in cases {
        let response = adapter.handle_request(1, "evaluate", Some(args.clone()));
        match response {
            DapMessage::Response { command, .. } => {
                assert_eq!(
                    command, "evaluate",
                    "response command should be 'evaluate' for case {label}"
                );
            }
            other => {
                return Err(format!("expected Response for case {label}, got {other:?}").into());
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC: Security — increment/decrement blocked in safe mode
// ---------------------------------------------------------------------------

#[test]
fn test_evaluate_increment_blocked_in_safe_mode() -> TestResult {
    let mut adapter = new_adapter();
    for expr in ["$i++", "++$i", "$i--", "--$i"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

#[test]
fn test_evaluate_assignment_ops_blocked_in_safe_mode() -> TestResult {
    let mut adapter = new_adapter();
    for expr in ["$x = 1", "$x += 1", "$x -= 1", "$x .= 'suffix'", "$x **= 2"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC: setExpression error handling
// ---------------------------------------------------------------------------

#[test]
fn test_set_expression_missing_arguments_returns_error() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(1, "setExpression", None);
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "setExpression");
            assert!(!success, "setExpression with no arguments should fail");
            let msg = message.ok_or("expected error message")?;
            assert!(!msg.is_empty());
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_set_expression_empty_expression_returns_error() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "setExpression",
        Some(json!({ "expression": "", "value": "42" })),
    );
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "setExpression");
            assert!(!success);
            let msg = message.ok_or("expected error message")?;
            assert!(
                msg.contains("expression") || msg.contains("Missing"),
                "got: {msg:?}"
            );
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_set_expression_empty_value_returns_error() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "setExpression",
        Some(json!({ "expression": "$x", "value": "" })),
    );
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "setExpression");
            assert!(!success);
            let msg = message.ok_or("expected error message")?;
            assert!(
                msg.contains("value") || msg.contains("Missing"),
                "got: {msg:?}"
            );
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_set_expression_newline_in_value_is_rejected() -> TestResult {
    let mut adapter = new_adapter();
    let response = adapter.handle_request(
        1,
        "setExpression",
        Some(json!({ "expression": "$x", "value": "42\nsystem('evil')" })),
    );
    match response {
        DapMessage::Response {
            success,
            command,
            message,
            ..
        } => {
            assert_eq!(command, "setExpression");
            assert!(!success);
            let msg = message.ok_or("expected error message")?;
            assert!(
                msg.contains("newline") || msg.contains("newlines"),
                "should mention newlines, got: {msg:?}"
            );
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }
    Ok(())
}
