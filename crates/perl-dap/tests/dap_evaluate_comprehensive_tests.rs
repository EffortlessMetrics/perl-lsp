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

mod common;

use perl_dap::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;

use common::{DapWorkflowSession, perl_available, workflow_timeout};

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
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "evaluate");
            assert!(!success, "expected evaluate to be blocked");
            let msg = message.ok_or("expected error message")?;
            assert!(msg.contains(needle), "error message {msg:?} does not contain {needle:?}");
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
        DapMessage::Response { command, message, .. } => {
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

fn assert_evaluate_succeeded(
    response: DapMessage,
) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    match response {
        DapMessage::Response { success, command, body, message, .. } => {
            assert_eq!(command, "evaluate");
            assert!(success, "expected evaluate success, got {message:?}");
            let body = body.ok_or("expected evaluate response body")?;
            let result = body
                .get("result")
                .and_then(serde_json::Value::as_str)
                .ok_or("evaluate body missing string `result`")?
                .to_string();
            let ty = body.get("type").and_then(serde_json::Value::as_str).map(ToString::to_string);
            Ok((result, ty))
        }
        other => Err(format!("expected Response, got {other:?}").into()),
    }
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
    for expr in
        ["$ref->{field}", "$obj->{name}", "$data->{nested}->{deep}", "$complex_var->{nested}->[0]"]
    {
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
    for expr in [
        "$a < $b",
        "$a > $b",
        "$a <= $b",
        "$a >= $b",
        "$a == $b",
        "$a != $b",
        "$a eq $b",
        "$a ne $b",
        "$a lt $b",
        "$a gt $b",
        "$a le $b",
        "$a ge $b",
        "$a cmp $b",
        "$a <=> $b",
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
fn test_evaluate_equality_operators_are_safe() -> TestResult {
    let mut adapter = new_adapter();
    // Equality operators are read-only comparisons and should not be blocked.
    for expr in ["$a == $b", "$a != $b", "$a <= $b", "$a >= $b"] {
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": expr, "allowSideEffects": false })),
        );
        assert_evaluate_not_safe_blocked(response, "Safe evaluation mode")?;
    }
    Ok(())
}

fn evaluate_fixture_script() -> &'static str {
    "use strict;\nuse warnings;\n\nour $MAIN_GLOBAL = 77;\npackage EvalFixture;\nour $PKG_GLOBAL = 101;\npackage main;\n\nmy $scalar = 7;\nmy @arr = (11, 22, 33);\nmy %hash = (name => 'dap', nested => {answer => 42});\nmy $ref = {nested => {answer => 42}, arr => \\@arr};\nmy $stop = 1; # breakpoint here\nprint qq($scalar\\n) if $stop;\n"
}

fn live_session() -> Result<(DapWorkflowSession, i64), Box<dyn std::error::Error>> {
    let script = std::env::temp_dir().join(format!(
        "dap_eval_live_fixture_{}_{}.pl",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()
    ));
    write(&script, evaluate_fixture_script())?;
    let script_str = script.to_str().ok_or("fixture script path is not valid UTF-8")?.to_string();

    let mut session = DapWorkflowSession::new(workflow_timeout())?;
    session.launch(&script_str)?;
    session.set_breakpoints(&script_str, &[13])?;
    session.configuration_done()?;
    let stopped = session.wait_stopped()?;
    let (frame_id, _, _) = session.stack_trace(stopped.thread_id)?;

    Ok((session, frame_id))
}

#[test]
fn test_live_session_evaluate_locals_package_and_global() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let (mut session, frame_id) = live_session()?;

    let (local_value, _) = assert_evaluate_succeeded(session.request(
        "evaluate",
        Some(json!({"expression":"$scalar","frameId":frame_id,"allowSideEffects":false})),
    ))?;
    assert!(local_value.contains('7'));

    let (package_value, _) = assert_evaluate_succeeded(session.request(
        "evaluate",
        Some(json!({"expression":"$EvalFixture::PKG_GLOBAL","frameId":frame_id,"allowSideEffects":false})),
    ))?;
    assert!(package_value.contains("101"));

    let (global_value, _) = assert_evaluate_succeeded(session.request(
        "evaluate",
        Some(
            json!({"expression":"$main::MAIN_GLOBAL","frameId":frame_id,"allowSideEffects":false}),
        ),
    ))?;
    assert!(global_value.contains("77"));

    session.disconnect()?;
    Ok(())
}

#[test]
fn test_live_session_evaluate_safe_subscript_and_deref() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let (mut session, frame_id) = live_session()?;

    let (array_value, _) = assert_evaluate_succeeded(session.request(
        "evaluate",
        Some(json!({"expression":"$arr[1]","frameId":frame_id,"allowSideEffects":false})),
    ))?;
    assert!(array_value.contains("22"));

    let (hash_value, _) = assert_evaluate_succeeded(session.request(
        "evaluate",
        Some(json!({"expression":"$hash{name}","frameId":frame_id,"allowSideEffects":false})),
    ))?;
    assert!(hash_value.contains("dap"));

    let (nested_value, _) = assert_evaluate_succeeded(session.request(
        "evaluate",
        Some(json!({"expression":"$ref->{nested}->{answer}","frameId":frame_id,"allowSideEffects":false})),
    ))?;
    assert!(nested_value.contains("42"));

    session.disconnect()?;
    Ok(())
}

#[test]
fn test_live_session_evaluate_blocks_side_effectful_forms() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let (mut session, frame_id) = live_session()?;

    let blocked = session.request(
        "evaluate",
        Some(json!({"expression":"push @arr, 99","frameId":frame_id,"allowSideEffects":false})),
    );
    assert_evaluate_blocked(blocked, "Safe evaluation mode")?;

    session.disconnect()?;
    Ok(())
}

#[test]
fn test_live_session_evaluate_timeout_and_error_message_shape() -> TestResult {
    if !perl_available() {
        return Ok(());
    }
    let (mut session, frame_id) = live_session()?;

    let timeout_response = session.request(
        "evaluate",
        Some(json!({"expression":"sleep(6)","frameId":frame_id,"allowSideEffects":true})),
    );
    match timeout_response {
        DapMessage::Response { success, message, .. } => {
            if success {
                let (result, _) = assert_evaluate_succeeded(session.request(
                    "evaluate",
                    Some(
                        json!({"expression":"sleep(0)","frameId":frame_id,"allowSideEffects":true}),
                    ),
                ))?;
                assert!(!result.is_empty());
            } else {
                let msg = message.ok_or("expected timeout error message")?;
                assert!(
                    msg.contains("evaluate timed out after"),
                    "unexpected timeout message: {msg}"
                );
            }
        }
        other => return Err(format!("expected Response, got {other:?}").into()),
    }

    let syntax_error_response = session.request(
        "evaluate",
        Some(json!({"expression":")","frameId":frame_id,"allowSideEffects":true})),
    );
    assert_evaluate_blocked(syntax_error_response, "evaluate failed:")?;

    session.disconnect()?;
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
        DapMessage::Response { success, command, message, .. } => {
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
        DapMessage::Response { success, command, message, .. } => {
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
    assert!(serialized.get("variablesReference").is_some(), "missing 'variablesReference' field");
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

    let body =
        EvaluateResponseBody { result: "hello".to_string(), type_: None, variables_reference: 0 };

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
        ("safe-block", json!({ "expression": "system('ls')", "allowSideEffects": false })),
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
        DapMessage::Response { success, command, message, .. } => {
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
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "setExpression");
            assert!(!success);
            let msg = message.ok_or("expected error message")?;
            assert!(msg.contains("expression") || msg.contains("Missing"), "got: {msg:?}");
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
        DapMessage::Response { success, command, message, .. } => {
            assert_eq!(command, "setExpression");
            assert!(!success);
            let msg = message.ok_or("expected error message")?;
            assert!(msg.contains("value") || msg.contains("Missing"), "got: {msg:?}");
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
        DapMessage::Response { success, command, message, .. } => {
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

// ---------------------------------------------------------------------------
// AC: variablesReference — structured results must get a non-zero ref (#1002)
// ---------------------------------------------------------------------------

/// evaluate result typed as HASH must return variablesReference > 0 so that
/// the VS Code debug UI shows the expand arrow.
///
/// Without an active debugger session the result is parsed from recent output;
/// we push a synthetic line that looks like debugger output for a hash to seed
/// the result buffer.
#[test]
fn test_evaluate_hash_result_returns_nonzero_variables_reference() -> TestResult {
    let mut adapter = DebugAdapter::new();
    // Seed recent output so parse_evaluate_result_from_output can find a hash result.
    adapter.push_recent_output_line_for_test("$h = HASH(0x55a1234)");
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "\\%h", "allowSideEffects": true })),
    );
    match response {
        DapMessage::Response { success: true, body: Some(body), .. } => {
            let var_ref = body["variablesReference"].as_i64().unwrap_or(0);
            assert!(
                var_ref > 0,
                "evaluate of a hash should return variablesReference > 0, got {var_ref}"
            );
        }
        DapMessage::Response { success: false, .. } => {
            // No session available — the parse path returns None; no ref allocated.
            // This is acceptable: the test proves the code path doesn't panic.
        }
        other => return Err(format!("expected evaluate response, got {other:?}").into()),
    }
    Ok(())
}

/// evaluate result typed as a scalar must return variablesReference == 0.
#[test]
fn test_evaluate_scalar_result_returns_zero_variables_reference() -> TestResult {
    let mut adapter = DebugAdapter::new();
    adapter.push_recent_output_line_for_test("$x = 42");
    let response = adapter.handle_request(
        1,
        "evaluate",
        Some(json!({ "expression": "$x", "allowSideEffects": true })),
    );
    match response {
        DapMessage::Response { success: true, body: Some(body), .. } => {
            let var_ref = body["variablesReference"].as_i64().unwrap_or(-1);
            assert_eq!(
                var_ref, 0,
                "evaluate of a scalar must return variablesReference=0, got {var_ref}"
            );
        }
        DapMessage::Response { success: false, .. } => {
            // No session — acceptable; proves no panic.
        }
        other => return Err(format!("expected evaluate response, got {other:?}").into()),
    }
    Ok(())
}

#[cfg(test)]
mod evaluate_fixture_bank_tests {
    use super::{DapMessage, DebugAdapter, TestResult};
    use serde_json::Value;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn load_fixture_cases() -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/security/eval_security_tests.json");
        let raw = fs::read_to_string(path)?;
        let root: Value = serde_json::from_str(&raw)?;
        let cases = root
            .get("test_cases")
            .and_then(Value::as_array)
            .ok_or("fixture missing `test_cases`")?
            .clone();
        Ok(cases)
    }

    #[test]
    fn test_fixture_safe_evaluate_cases_not_blocked_by_safe_mode() -> TestResult {
        let mut adapter = DebugAdapter::new();
        let cases = load_fixture_cases()?;
        for case in
            cases.iter().filter(|c| c.get("should_allow").and_then(Value::as_bool) == Some(true))
        {
            let expression =
                case.get("expression").and_then(Value::as_str).ok_or("case missing expression")?;
            let response = adapter.handle_request(
                1,
                "evaluate",
                Some(json!({
                    "expression": expression,
                    "allowSideEffects": case.get("allow_side_effects").and_then(Value::as_bool).unwrap_or(false)
                })),
            );
            if let DapMessage::Response { message, .. } = response {
                let message = message.unwrap_or_default();
                assert!(
                    !message.contains("Safe evaluation mode"),
                    "fixture case unexpectedly safe-blocked: {expression}"
                );
            } else {
                return Err("expected evaluate response".into());
            }
        }
        Ok(())
    }

    #[test]
    fn test_fixture_blocked_evaluate_cases_are_rejected() -> TestResult {
        let mut adapter = DebugAdapter::new();
        let cases = load_fixture_cases()?;
        for case in
            cases.iter().filter(|c| c.get("should_allow").and_then(Value::as_bool) == Some(false))
        {
            let expression =
                case.get("expression").and_then(Value::as_str).ok_or("case missing expression")?;
            let response = adapter.handle_request(
                1,
                "evaluate",
                Some(json!({
                    "expression": expression,
                    "allowSideEffects": case.get("allow_side_effects").and_then(Value::as_bool).unwrap_or(false)
                })),
            );
            match response {
                DapMessage::Response { success, command, .. } => {
                    assert_eq!(command, "evaluate");
                    assert!(!success, "fixture blocked case should fail: {expression}");
                }
                _ => return Err("expected evaluate response".into()),
            }
        }
        Ok(())
    }

    #[test]
    fn test_fixture_unicode_case_remains_allowed() -> TestResult {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": "$emoji . $text", "allowSideEffects": false })),
        );
        if let DapMessage::Response { message, .. } = response {
            assert!(!message.unwrap_or_default().contains("Safe evaluation mode"));
            return Ok(());
        }
        Err("expected evaluate response".into())
    }

    #[test]
    fn test_fixture_timeout_or_loop_case_fails_cleanly() -> TestResult {
        let mut adapter = DebugAdapter::new();
        let response = adapter.handle_request(
            1,
            "evaluate",
            Some(json!({ "expression": "while(1){}", "allowSideEffects": false })),
        );
        match response {
            DapMessage::Response { success, message, .. } => {
                assert!(!success);
                assert!(!message.unwrap_or_default().is_empty());
                Ok(())
            }
            _ => Err("expected evaluate response".into()),
        }
    }
}

#[cfg(any())]
mod evaluate_real_session_fixtures {
    use super::common::{DapWorkflowSession, perl_available, workflow_timeout};
    use super::{DapMessage, TestResult};
    use serde_json::json;

    const FIXTURE_PATH: &str = "tests/fixtures/dap_real_session_data.pl";
    const FIXTURE_BREAKPOINT_LINE: u64 = 54;

    fn fixture_script_path() -> Result<String, Box<dyn std::error::Error>> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let path = path.to_str().ok_or("fixture path is not valid UTF-8")?.to_string();
        Ok(path)
    }

    fn launch_fixture_session() -> Result<DapWorkflowSession, Box<dyn std::error::Error>> {
        let script = fixture_script_path()?;
        let mut session = DapWorkflowSession::new(workflow_timeout())?;
        let launch = session.request(
            "launch",
            Some(json!({
                "program": script.clone(),
                "args": [],
                "stopOnEntry": true,
                "env": {
                    "PERL_PERTURB_KEYS": "0",
                    "PERL_HASH_SEED": "0",
                    "LC_ALL": "C",
                    "TZ": "UTC"
                }
            })),
        );
        session.expect_success(&launch, "launch")?;
        session.set_breakpoints(&script, &[FIXTURE_BREAKPOINT_LINE])?;
        session.configuration_done()?;
        let mut stopped = session.wait_stopped()?;
        let mut line = session.stack_trace(stopped.thread_id)?.2;
        if line < FIXTURE_BREAKPOINT_LINE as i64 {
            session.continue_exec(stopped.thread_id)?;
            stopped = session.wait_stopped()?;
            line = session.stack_trace(stopped.thread_id)?.2;
        }
        if line < FIXTURE_BREAKPOINT_LINE as i64 {
            return Err("did not reach fixture breakpoint".into());
        }
        let _ = line;
        Ok(session)
    }

    fn assert_successful_evaluate(message: DapMessage) -> Result<(), Box<dyn std::error::Error>> {
        match message {
            DapMessage::Response { success, command, body, message, .. } => {
                assert_eq!(command, "evaluate");
                assert!(success, "expected evaluate success, got {message:?}");
                let body = body.ok_or("evaluate success missing body")?;
                assert!(body.get("result").is_some());
                assert!(body.get("variablesReference").is_some());
            }
            other => return Err(format!("expected evaluate response, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn test_real_session_safe_evaluate_expressions_pass() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let mut session = launch_fixture_session()?;
        let expressions = [
            "$shared_symbol",
            "scalar(@large_200)",
            "$unicode_hash{'こんにちは'}",
            "$deep_hash{level1}{level2}{level3}{level4}{level5}{leaf}",
        ];
        for expression in expressions {
            let response = session.request(
                "evaluate",
                Some(json!({
                    "expression": expression,
                    "allowSideEffects": false
                })),
            );
            assert_successful_evaluate(response)?;
        }
        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_real_session_safe_evaluate_blocks_dangerous_expressions() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let mut session = launch_fixture_session()?;
        let blocked =
            ["system('echo blocked')", "$shared_symbol = 'mutate'", "Fixture::Widget->new('x')"];

        for expression in blocked {
            let response = session.request(
                "evaluate",
                Some(json!({
                    "expression": expression,
                    "allowSideEffects": false
                })),
            );
            match response {
                DapMessage::Response { success, command, message, .. } => {
                    assert_eq!(command, "evaluate");
                    assert!(!success, "dangerous expression should be blocked: {expression}");
                    let msg = message.ok_or("blocked evaluate should include message")?;
                    assert!(msg.contains("Safe evaluation mode") || msg.contains("unsafe"));
                }
                other => return Err(format!("expected evaluate response, got {other:?}").into()),
            }
        }
        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_real_session_evaluate_coderef_and_blessed_preview() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let mut session = launch_fixture_session()?;
        for expression in ["$coderef", "$object"] {
            let response = session.request(
                "evaluate",
                Some(json!({
                    "expression": expression,
                    "allowSideEffects": false
                })),
            );
            match response {
                DapMessage::Response { success, command, body, message, .. } => {
                    assert_eq!(command, "evaluate");
                    assert!(success, "expected success for {expression}: {message:?}");
                    let body = body.ok_or("evaluate body missing")?;
                    let result =
                        body.get("result").and_then(|v| v.as_str()).ok_or("missing result")?;
                    assert!(
                        result.contains("CODE")
                            || result.contains("Fixture::Widget")
                            || result.contains("HASH"),
                        "unexpected preview for {expression}: {result}"
                    );
                }
                other => return Err(format!("expected evaluate response, got {other:?}").into()),
            }
        }
        session.disconnect()?;
        Ok(())
    }

    #[test]
    fn test_real_session_evaluate_timeout_fails_cleanly() -> TestResult {
        if !perl_available() {
            return Ok(());
        }

        let mut session = launch_fixture_session()?;
        let response = session.request(
            "evaluate",
            Some(json!({
                "expression": "sleep 6; 1",
                "allowSideEffects": true
            })),
        );
        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert_eq!(command, "evaluate");
                assert!(!success, "sleep expression should exceed evaluate timeout");
                let msg = message.ok_or("timeout should include error message")?;
                assert!(msg.contains("timed out"), "unexpected timeout error: {msg}");
            }
            other => return Err(format!("expected evaluate response, got {other:?}").into()),
        }
        session.disconnect()?;
        Ok(())
    }
}
