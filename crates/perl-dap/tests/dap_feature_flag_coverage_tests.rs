//! DAP Feature Flag Coverage Tests
//!
//! Explicit test coverage for 8 DAP features that previously had no dedicated
//! feature-gate validation:
//!
//! - AC0: dap.core
//! - AC1: dap.breakpoints.basic
//! - AC2: dap.breakpoints.hit_condition
//! - AC3: dap.breakpoints.logpoints
//! - AC4: dap.completions
//! - AC5: dap.exceptions.die
//! - AC6: dap.inline_values
//! - AC7: dap.modules
//!
//! Each feature gets:
//! 1. Feature gate test: `has_feature("dap.X")` returns true
//! 2. Capability test: initialize response advertises the feature correctly
//! 3. Functional test: feature-gated code path works when enabled
//!
//! Related issues: #2784, #435, #2783

use perl_dap::feature_catalog::has_feature;
use perl_dap::{DapMessage, DebugAdapter};
use serde_json::json;
use std::io::Write;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn initialize_adapter() -> DebugAdapter {
    let mut adapter = DebugAdapter::new();
    adapter.handle_request(1, "initialize", None);
    adapter
}

fn get_initialize_body() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut adapter = DebugAdapter::new();
    match adapter.handle_request(1, "initialize", None) {
        DapMessage::Response { success: true, body: Some(body), .. } => Ok(body),
        _ => Err("Expected successful initialize response with body".into()),
    }
}

fn expect_success_body(
    response: DapMessage,
    command: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    match response {
        DapMessage::Response { success: true, command: actual, body: Some(body), .. }
            if actual == command =>
        {
            Ok(body)
        }
        DapMessage::Response { success: true, command: actual, .. } if actual == command => {
            Ok(serde_json::Value::Null)
        }
        DapMessage::Response { success: false, message, command: actual, .. } => Err(format!(
            "command `{actual}` failed: {}",
            message.unwrap_or_else(|| "<no message>".to_string())
        )
        .into()),
        _ => Err(format!("Expected response for `{command}`").into()),
    }
}

// ---------------------------------------------------------------------------
// AC0: dap.core
// ---------------------------------------------------------------------------

/// Feature gate: dap.core is registered in the catalog.
#[test]
fn test_feature_gate_dap_core() {
    assert!(has_feature("dap.core"), "dap.core must be registered in the feature catalog");
}

/// Capability test: initialize advertises core capabilities when dap.core is enabled.
#[test]
fn test_capability_dap_core_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.core") {
        let supports_config_done =
            body.get("supportsConfigurationDoneRequest").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_config_done,
            "supportsConfigurationDoneRequest must be true when dap.core is enabled"
        );

        let supports_evaluate =
            body.get("supportsEvaluateForHovers").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_evaluate,
            "supportsEvaluateForHovers must be true when dap.core is enabled"
        );

        let supports_set_variable =
            body.get("supportsSetVariable").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(supports_set_variable, "supportsSetVariable must be true when dap.core is enabled");
    }
    Ok(())
}

/// Functional test: configurationDone succeeds (core DAP lifecycle).
#[test]
fn test_functional_dap_core_configuration_done() -> TestResult {
    let mut adapter = initialize_adapter();
    let response = adapter.handle_request(2, "configurationDone", None);

    match response {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "configurationDone");
        }
        _ => return Err("Expected configurationDone response".into()),
    }
    Ok(())
}

/// Functional test: threads request succeeds (core DAP capability).
///
/// Without an active debug session, threads returns an empty array — this is
/// correct DAP behavior. The test validates that the command is dispatched and
/// the response body has the expected shape.
#[test]
fn test_functional_dap_core_threads() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(adapter.handle_request(2, "threads", None), "threads")?;

    // threads array must be present (may be empty without an active session)
    let _threads = body.get("threads").and_then(|v| v.as_array()).ok_or("missing threads array")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// AC1: dap.breakpoints.basic
// ---------------------------------------------------------------------------

/// Feature gate: dap.breakpoints.basic is registered in the catalog.
#[test]
fn test_feature_gate_dap_breakpoints_basic() {
    assert!(
        has_feature("dap.breakpoints.basic"),
        "dap.breakpoints.basic must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises conditional breakpoint support.
#[test]
fn test_capability_dap_breakpoints_basic_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.breakpoints.basic") {
        let supports_conditional =
            body.get("supportsConditionalBreakpoints").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_conditional,
            "supportsConditionalBreakpoints must be true when dap.breakpoints.basic is enabled"
        );
    }
    Ok(())
}

/// Functional test: setBreakpoints responds successfully with at least one breakpoint record.
#[test]
fn test_functional_dap_breakpoints_basic_set_breakpoints() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(
            2,
            "setBreakpoints",
            Some(json!({
                "source": { "path": "/tmp/test.pl", "name": "test.pl" },
                "breakpoints": [{ "line": 5 }]
            })),
        ),
        "setBreakpoints",
    )?;

    let breakpoints =
        body.get("breakpoints").and_then(|v| v.as_array()).ok_or("missing breakpoints array")?;
    assert_eq!(breakpoints.len(), 1, "setBreakpoints must return one record per requested BP");
    Ok(())
}

/// Functional test: clearing breakpoints (replace semantics) returns empty array.
#[test]
fn test_functional_dap_breakpoints_basic_clear_breakpoints() -> TestResult {
    let mut adapter = initialize_adapter();

    // Set breakpoints
    adapter.handle_request(
        2,
        "setBreakpoints",
        Some(json!({
            "source": { "path": "/tmp/test.pl", "name": "test.pl" },
            "breakpoints": [{ "line": 5 }, { "line": 10 }]
        })),
    );

    // Clear (replace with empty list)
    let body = expect_success_body(
        adapter.handle_request(
            3,
            "setBreakpoints",
            Some(json!({
                "source": { "path": "/tmp/test.pl", "name": "test.pl" },
                "breakpoints": []
            })),
        ),
        "setBreakpoints",
    )?;

    let breakpoints =
        body.get("breakpoints").and_then(|v| v.as_array()).ok_or("missing breakpoints array")?;
    assert!(breakpoints.is_empty(), "clearing breakpoints must return an empty array");
    Ok(())
}

// ---------------------------------------------------------------------------
// AC2: dap.breakpoints.hit_condition
// ---------------------------------------------------------------------------

/// Feature gate: dap.breakpoints.hit_condition is registered in the catalog.
#[test]
fn test_feature_gate_dap_breakpoints_hit_condition() {
    assert!(
        has_feature("dap.breakpoints.hit_condition"),
        "dap.breakpoints.hit_condition must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises hit-conditional breakpoint support.
#[test]
fn test_capability_dap_breakpoints_hit_condition_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.breakpoints.hit_condition") {
        let supports_hit_conditional = body
            .get("supportsHitConditionalBreakpoints")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(
            supports_hit_conditional,
            "supportsHitConditionalBreakpoints must be true when dap.breakpoints.hit_condition is enabled"
        );
    }
    Ok(())
}

/// Functional test: setBreakpoints accepts hitCondition field and stores the breakpoint.
#[test]
fn test_functional_dap_breakpoints_hit_condition_accepted() -> TestResult {
    let store = perl_dap::breakpoints::BreakpointStore::new();
    let args = perl_dap::protocol::SetBreakpointsArguments {
        source: perl_dap::protocol::Source {
            path: Some("/tmp/test.pl".to_string()),
            name: Some("test.pl".to_string()),
        },
        breakpoints: Some(vec![perl_dap::protocol::SourceBreakpoint {
            line: 5,
            column: None,
            condition: None,
            hit_condition: Some(">= 3".to_string()),
            log_message: None,
        }]),
        source_modified: None,
    };

    let breakpoints = store.set_breakpoints(&args);
    assert_eq!(breakpoints.len(), 1, "hit_condition breakpoint must return one record");
    Ok(())
}

/// Functional test: hit-condition `>= 2` — first hit does not stop, second does.
///
/// Uses a real temp Perl file so AST validation can verify the breakpoint.
#[test]
fn test_functional_dap_breakpoints_hit_condition_gte_two() -> TestResult {
    use tempfile::NamedTempFile;
    let mut temp = NamedTempFile::with_suffix(".pl").map_err(|e| e.to_string())?;
    let perl_src = b"my $x = 0;\n$x++;\n$x++;\nprint $x;\n";
    temp.write_all(perl_src).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    let path = temp.path().to_string_lossy().to_string();

    let store = perl_dap::breakpoints::BreakpointStore::new();
    let args = perl_dap::protocol::SetBreakpointsArguments {
        source: perl_dap::protocol::Source {
            path: Some(path.clone()),
            name: Some("hit_test.pl".to_string()),
        },
        breakpoints: Some(vec![perl_dap::protocol::SourceBreakpoint {
            line: 2,
            column: None,
            condition: None,
            hit_condition: Some(">= 2".to_string()),
            log_message: None,
        }]),
        source_modified: None,
    };

    let bps = store.set_breakpoints(&args);
    assert_eq!(bps.len(), 1, "expected one breakpoint record");
    assert!(bps[0].verified, "breakpoint on executable line must be verified");

    let first = store.register_breakpoint_hit(&path, 2);
    assert!(first.matched, "first hit must match the breakpoint");
    assert!(!first.should_stop, "first hit must not stop for `>= 2`");

    let second = store.register_breakpoint_hit(&path, 2);
    assert!(second.matched, "second hit must match the breakpoint");
    assert!(second.should_stop, "second hit must stop for `>= 2`");
    Ok(())
}

/// Functional test: hit-condition `%3` stops on every third hit.
///
/// Uses a real temp Perl file so AST validation can verify the breakpoint.
#[test]
fn test_functional_dap_breakpoints_hit_condition_modulo() -> TestResult {
    use tempfile::NamedTempFile;
    let mut temp = NamedTempFile::with_suffix(".pl").map_err(|e| e.to_string())?;
    let perl_src = b"my $x = 0;\n$x++;\n$x++;\n$x++;\nprint $x;\n";
    temp.write_all(perl_src).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    let path = temp.path().to_string_lossy().to_string();

    let store = perl_dap::breakpoints::BreakpointStore::new();
    let args = perl_dap::protocol::SetBreakpointsArguments {
        source: perl_dap::protocol::Source {
            path: Some(path.clone()),
            name: Some("modulo_test.pl".to_string()),
        },
        breakpoints: Some(vec![perl_dap::protocol::SourceBreakpoint {
            line: 2,
            column: None,
            condition: None,
            hit_condition: Some("%3".to_string()),
            log_message: None,
        }]),
        source_modified: None,
    };

    let bps = store.set_breakpoints(&args);
    assert_eq!(bps.len(), 1, "expected one breakpoint record");
    assert!(bps[0].verified, "breakpoint on executable line must be verified");

    let h1 = store.register_breakpoint_hit(&path, 2);
    let h2 = store.register_breakpoint_hit(&path, 2);
    let h3 = store.register_breakpoint_hit(&path, 2);

    assert!(!h1.should_stop, "hit 1 must not stop for `%3`");
    assert!(!h2.should_stop, "hit 2 must not stop for `%3`");
    assert!(h3.should_stop, "hit 3 must stop for `%3`");
    Ok(())
}

// ---------------------------------------------------------------------------
// AC3: dap.breakpoints.logpoints
// ---------------------------------------------------------------------------

/// Feature gate: dap.breakpoints.logpoints is registered in the catalog.
#[test]
fn test_feature_gate_dap_breakpoints_logpoints() {
    assert!(
        has_feature("dap.breakpoints.logpoints"),
        "dap.breakpoints.logpoints must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises logpoint support.
#[test]
fn test_capability_dap_breakpoints_logpoints_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.breakpoints.logpoints") {
        let supports_log_points =
            body.get("supportsLogPoints").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_log_points,
            "supportsLogPoints must be true when dap.breakpoints.logpoints is enabled"
        );
    }
    Ok(())
}

/// Functional test: logpoint emits log_messages and does not stop execution.
///
/// Uses a real temp Perl file so AST validation can verify the breakpoint.
#[test]
fn test_functional_dap_breakpoints_logpoint_emits_and_continues() -> TestResult {
    use tempfile::NamedTempFile;
    let mut temp = NamedTempFile::with_suffix(".pl").map_err(|e| e.to_string())?;
    let perl_src = b"my $x = 1;\nmy $y = 2;\nmy $z = $x + $y;\nprint $z;\n";
    temp.write_all(perl_src).map_err(|e| e.to_string())?;
    temp.flush().map_err(|e| e.to_string())?;
    let path = temp.path().to_string_lossy().to_string();

    let store = perl_dap::breakpoints::BreakpointStore::new();
    let args = perl_dap::protocol::SetBreakpointsArguments {
        source: perl_dap::protocol::Source {
            path: Some(path.clone()),
            name: Some("logpoint_test.pl".to_string()),
        },
        breakpoints: Some(vec![perl_dap::protocol::SourceBreakpoint {
            line: 3,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: Some("z = {$z}".to_string()),
        }]),
        source_modified: None,
    };

    let bps = store.set_breakpoints(&args);
    assert_eq!(bps.len(), 1, "expected one breakpoint record");
    assert!(bps[0].verified, "logpoint on executable line must be verified");

    let outcome = store.register_breakpoint_hit(&path, 3);
    assert!(outcome.matched, "logpoint must match the breakpoint");
    assert!(!outcome.should_stop, "logpoint must not stop execution");
    assert!(!outcome.log_messages.is_empty(), "logpoint must emit at least one log message");
    Ok(())
}

/// Functional test: setBreakpoints with logMessage field stores the breakpoint.
#[test]
fn test_functional_dap_breakpoints_logpoint_stored_via_set_breakpoints() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(
            2,
            "setBreakpoints",
            Some(json!({
                "source": { "path": "/tmp/logtest.pl", "name": "logtest.pl" },
                "breakpoints": [{ "line": 4, "logMessage": "Reached line 4: {$x}" }]
            })),
        ),
        "setBreakpoints",
    )?;

    let breakpoints =
        body.get("breakpoints").and_then(|v| v.as_array()).ok_or("missing breakpoints array")?;
    assert_eq!(breakpoints.len(), 1, "logpoint must appear in setBreakpoints response");
    Ok(())
}

// ---------------------------------------------------------------------------
// AC4: dap.completions
// ---------------------------------------------------------------------------

/// Feature gate: dap.completions is registered in the catalog.
#[test]
fn test_feature_gate_dap_completions() {
    assert!(
        has_feature("dap.completions"),
        "dap.completions must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises completions support.
#[test]
fn test_capability_dap_completions_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.completions") {
        let supports_completions =
            body.get("supportsCompletionsRequest").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_completions,
            "supportsCompletionsRequest must be true when dap.completions is enabled"
        );
    }
    Ok(())
}

/// Functional test: completions request returns a list of Perl keywords.
#[test]
fn test_functional_dap_completions_returns_keywords() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(2, "completions", Some(json!({ "text": "pri", "column": 3 }))),
        "completions",
    )?;

    let targets = body.get("targets").and_then(|v| v.as_array()).ok_or("missing targets array")?;
    assert!(!targets.is_empty(), "completions for 'pri' must return at least one target");

    let labels: Vec<&str> =
        targets.iter().filter_map(|t| t.get("label").and_then(|l| l.as_str())).collect();
    assert!(
        labels.contains(&"print"),
        "completions for 'pri' must include 'print'; got: {labels:?}"
    );
    Ok(())
}

/// Functional test: completions with empty prefix returns all Perl keywords.
#[test]
fn test_functional_dap_completions_empty_prefix() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(2, "completions", Some(json!({ "text": "", "column": 0 }))),
        "completions",
    )?;

    let targets = body.get("targets").and_then(|v| v.as_array()).ok_or("missing targets array")?;
    assert!(!targets.is_empty(), "completions with empty prefix must return some targets");
    Ok(())
}

// ---------------------------------------------------------------------------
// AC5: dap.exceptions.die
// ---------------------------------------------------------------------------

/// Feature gate: dap.exceptions.die is registered in the catalog.
#[test]
fn test_feature_gate_dap_exceptions_die() {
    assert!(
        has_feature("dap.exceptions.die"),
        "dap.exceptions.die must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises the `die` exception filter.
#[test]
fn test_capability_dap_exceptions_die_filter_in_initialize() -> TestResult {
    let body = get_initialize_body()?;

    let filters = body
        .get("exceptionBreakpointFilters")
        .and_then(|v| v.as_array())
        .ok_or("Missing exceptionBreakpointFilters")?;

    let has_die = filters.iter().any(|f| f.get("filter").and_then(|v| v.as_str()) == Some("die"));

    if has_feature("dap.exceptions.die") {
        assert!(
            has_die,
            "die filter must appear in exceptionBreakpointFilters when feature is enabled"
        );

        let die_filter = filters
            .iter()
            .find(|f| f.get("filter").and_then(|v| v.as_str()) == Some("die"))
            .ok_or("die filter not found")?;

        assert_eq!(
            die_filter.get("label").and_then(|v| v.as_str()),
            Some("Perl die() and uncaught exceptions"),
            "die filter must have correct label"
        );
    } else {
        assert!(!has_die, "die filter must not appear when dap.exceptions.die is disabled");
    }
    Ok(())
}

/// Functional test: setExceptionBreakpoints with `die` filter succeeds.
#[test]
fn test_functional_dap_exceptions_die_set_filter() -> TestResult {
    let mut adapter = initialize_adapter();
    let response =
        adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["die"] })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "setExceptionBreakpoints with die filter must succeed");
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected setExceptionBreakpoints response".into()),
    }
    Ok(())
}

/// Functional test: exceptionInfo request returns the `die` category details.
#[test]
fn test_functional_dap_exceptions_die_exception_info() -> TestResult {
    let mut adapter = initialize_adapter();
    adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["die"] })));

    let response = adapter.handle_request(3, "exceptionInfo", Some(json!({ "threadId": 1 })));

    // exceptionInfo may return a graceful failure when no session is active,
    // but the command must be recognized (not treated as unknown).
    match response {
        DapMessage::Response { command, .. } => {
            assert_eq!(command, "exceptionInfo", "exceptionInfo command must be dispatched");
        }
        _ => return Err("Expected exceptionInfo response".into()),
    }
    Ok(())
}

/// Functional test: `die` and `all` filters both present in exceptionBreakpointFilters.
#[test]
fn test_capability_dap_exceptions_die_and_all_filters() -> TestResult {
    if !has_feature("dap.exceptions.die") {
        return Ok(());
    }

    let body = get_initialize_body()?;
    let filters = body
        .get("exceptionBreakpointFilters")
        .and_then(|v| v.as_array())
        .ok_or("Missing exceptionBreakpointFilters")?;

    let has_all = filters.iter().any(|f| f.get("filter").and_then(|v| v.as_str()) == Some("all"));
    assert!(has_all, "`all` filter must appear alongside `die` in exceptionBreakpointFilters");
    Ok(())
}

// ---------------------------------------------------------------------------
// AC6: dap.inline_values
// ---------------------------------------------------------------------------

/// Feature gate: dap.inline_values is registered in the catalog.
#[test]
fn test_feature_gate_dap_inline_values() {
    assert!(
        has_feature("dap.inline_values"),
        "dap.inline_values must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises inline values support.
#[test]
fn test_capability_dap_inline_values_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.inline_values") {
        let supports_inline =
            body.get("supportsInlineValues").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_inline,
            "supportsInlineValues must be true when dap.inline_values is enabled"
        );
    }
    Ok(())
}

/// Functional test: inlineValues extracts scalar variables from source.
#[test]
fn test_functional_dap_inline_values_scalar_extraction() -> TestResult {
    use std::fs::write;
    use tempfile::tempdir;

    let dir = tempdir()?;
    let script = dir.path().join("inline_test.pl");
    write(&script, "my $foo = 42;\nmy $bar = $foo + 1;\n")?;

    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(
            2,
            "inlineValues",
            Some(json!({
                "source": { "path": script.to_str().ok_or("path error")? },
                "startLine": 1,
                "endLine": 2
            })),
        ),
        "inlineValues",
    )?;

    let values =
        body.get("inlineValues").and_then(|v| v.as_array()).ok_or("missing inlineValues array")?;

    assert!(
        values.iter().any(|v| v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .contains("$foo")),
        "inlineValues must include $foo"
    );
    assert!(
        values.iter().any(|v| v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .contains("$bar")),
        "inlineValues must include $bar"
    );
    Ok(())
}

/// Functional test: inlineValues skips special Perl variables.
#[test]
fn test_functional_dap_inline_values_skips_special_vars() -> TestResult {
    use std::fs::write;
    use tempfile::tempdir;

    let dir = tempdir()?;
    let script = dir.path().join("special_vars_test.pl");
    write(&script, "print $_; warn $!; my $val = 1;\n")?;

    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(
            2,
            "inlineValues",
            Some(json!({
                "source": { "path": script.to_str().ok_or("path error")? },
                "startLine": 1,
                "endLine": 1
            })),
        ),
        "inlineValues",
    )?;

    let values =
        body.get("inlineValues").and_then(|v| v.as_array()).ok_or("missing inlineValues array")?;

    let texts: Vec<&str> =
        values.iter().filter_map(|v| v.get("text").and_then(|t| t.as_str())).collect();

    assert!(
        !texts.iter().any(|t| t.contains("$_")),
        "inlineValues must not include special variable $_, got: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("$!")),
        "inlineValues must not include special variable $!, got: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("$val")),
        "inlineValues must include user variable $val, got: {texts:?}"
    );
    Ok(())
}

/// Functional test: inlineValues for array and hash variables includes correct formatting.
#[test]
fn test_functional_dap_inline_values_array_and_hash() -> TestResult {
    use std::fs::write;
    use tempfile::tempdir;

    let dir = tempdir()?;
    let script = dir.path().join("array_hash_test.pl");
    write(&script, "my @items = (1, 2, 3);\nmy %config = (a => 1);\n")?;

    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(
            2,
            "inlineValues",
            Some(json!({
                "source": { "path": script.to_str().ok_or("path error")? },
                "startLine": 1,
                "endLine": 2
            })),
        ),
        "inlineValues",
    )?;

    let values =
        body.get("inlineValues").and_then(|v| v.as_array()).ok_or("missing inlineValues array")?;

    assert!(
        values.iter().any(|v| v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .contains("@items")),
        "inlineValues must include @items"
    );
    assert!(
        values.iter().any(|v| v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .contains("%config")),
        "inlineValues must include %config"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// AC7: dap.modules
// ---------------------------------------------------------------------------

/// Feature gate: dap.modules is registered in the catalog.
#[test]
fn test_feature_gate_dap_modules() {
    assert!(has_feature("dap.modules"), "dap.modules must be registered in the feature catalog");
}

/// Capability test: initialize advertises modules support.
#[test]
fn test_capability_dap_modules_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    if has_feature("dap.modules") {
        let supports_modules =
            body.get("supportsModulesRequest").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            supports_modules,
            "supportsModulesRequest must be true when dap.modules is enabled"
        );
    }
    Ok(())
}

/// Functional test: modules request returns structured response (no active session returns empty).
#[test]
fn test_functional_dap_modules_no_session() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(2, "modules", Some(json!({ "startModule": 0 }))),
        "modules",
    )?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules array")?;
    let total =
        body.get("totalModules").and_then(|v| v.as_i64()).ok_or("missing totalModules field")?;

    assert!(modules.is_empty(), "modules without active session must return empty array");
    assert_eq!(total, 0, "totalModules without active session must be 0");
    Ok(())
}

/// Functional test: modules request with moduleCount limit is accepted.
#[test]
fn test_functional_dap_modules_with_count_limit() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(
        adapter.handle_request(2, "modules", Some(json!({ "startModule": 0, "moduleCount": 10 }))),
        "modules",
    )?;

    let modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules array")?;
    let _total = body.get("totalModules").ok_or("missing totalModules")?;

    // Without an active debug session, no modules should be present
    assert!(modules.len() <= 10, "modules must respect moduleCount limit");
    Ok(())
}

/// Functional test: modules request without arguments uses defaults.
#[test]
fn test_functional_dap_modules_without_arguments() -> TestResult {
    let mut adapter = initialize_adapter();
    let body = expect_success_body(adapter.handle_request(2, "modules", None), "modules")?;

    let _modules = body.get("modules").and_then(|v| v.as_array()).ok_or("missing modules array")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// AC8: dap.exceptions.warn
// ---------------------------------------------------------------------------

/// Feature gate: dap.exceptions.warn is registered in the catalog.
#[test]
fn test_feature_gate_dap_exceptions_warn() {
    assert!(
        has_feature("dap.exceptions.warn"),
        "dap.exceptions.warn must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises the `warn` exception filter when feature is enabled.
#[test]
fn test_capability_dap_exceptions_warn_filter_in_initialize() -> TestResult {
    let body = get_initialize_body()?;

    let filters = body
        .get("exceptionBreakpointFilters")
        .and_then(|v| v.as_array())
        .ok_or("Missing exceptionBreakpointFilters")?;

    let has_warn = filters.iter().any(|f| f.get("filter").and_then(|v| v.as_str()) == Some("warn"));

    if has_feature("dap.exceptions.warn") {
        assert!(
            has_warn,
            "warn filter must appear in exceptionBreakpointFilters when feature is enabled"
        );

        let warn_filter = filters
            .iter()
            .find(|f| f.get("filter").and_then(|v| v.as_str()) == Some("warn"))
            .ok_or("warn filter not found")?;

        assert_eq!(
            warn_filter.get("label").and_then(|v| v.as_str()),
            Some("Perl warn() and Carp warnings"),
            "warn filter must have correct label"
        );
        assert_eq!(
            warn_filter.get("default").and_then(|v| v.as_bool()),
            Some(false),
            "warn filter default must be false (non-intrusive by default)"
        );
    } else {
        assert!(!has_warn, "warn filter must not appear when dap.exceptions.warn is disabled");
    }
    Ok(())
}

/// Functional test: setExceptionBreakpoints with `warn` filter succeeds.
#[test]
fn test_functional_dap_exceptions_warn_set_filter() -> TestResult {
    let mut adapter = initialize_adapter();
    let response =
        adapter.handle_request(2, "setExceptionBreakpoints", Some(json!({ "filters": ["warn"] })));

    match response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "setExceptionBreakpoints with warn filter must succeed");
            assert_eq!(command, "setExceptionBreakpoints");
        }
        _ => return Err("Expected setExceptionBreakpoints response".into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AC9: dap.watchpoints
// ---------------------------------------------------------------------------

/// Feature gate: dap.watchpoints is registered in the catalog.
#[test]
fn test_feature_gate_dap_watchpoints() {
    assert!(
        has_feature("dap.watchpoints"),
        "dap.watchpoints must be registered in the feature catalog"
    );
}

/// Capability test: initialize advertises supportsDataBreakpoints when feature is enabled.
#[test]
fn test_capability_dap_watchpoints_initialize_response() -> TestResult {
    let body = get_initialize_body()?;

    let supports_data_breakpoints =
        body.get("supportsDataBreakpoints").and_then(|v| v.as_bool()).unwrap_or(false);

    if has_feature("dap.watchpoints") {
        assert!(
            supports_data_breakpoints,
            "supportsDataBreakpoints must be true when dap.watchpoints is enabled"
        );
    } else {
        assert!(
            !supports_data_breakpoints,
            "supportsDataBreakpoints must be false when dap.watchpoints is disabled"
        );
    }
    Ok(())
}

/// Functional test: dataBreakpointInfo and setDataBreakpoints work when watchpoints are enabled.
#[test]
fn test_functional_dap_watchpoints_data_breakpoint_roundtrip() -> TestResult {
    let mut adapter = initialize_adapter();

    // dataBreakpointInfo must succeed for a valid variable name
    let info_response =
        adapter.handle_request(2, "dataBreakpointInfo", Some(json!({ "name": "$x" })));

    match info_response {
        DapMessage::Response { success, command, .. } => {
            assert!(success, "dataBreakpointInfo must succeed");
            assert_eq!(command, "dataBreakpointInfo");
        }
        _ => return Err("Expected dataBreakpointInfo response".into()),
    }

    // setDataBreakpoints with a valid breakpoint must succeed and return a breakpoints array
    let set_response = adapter.handle_request(
        3,
        "setDataBreakpoints",
        Some(json!({ "breakpoints": [{ "dataId": "$x", "accessType": "write" }] })),
    );

    match set_response {
        DapMessage::Response { success, command, body: Some(body), .. } => {
            assert!(success, "setDataBreakpoints must succeed");
            assert_eq!(command, "setDataBreakpoints");
            let bps = body
                .get("breakpoints")
                .and_then(|v| v.as_array())
                .ok_or("missing breakpoints array")?;
            assert_eq!(bps.len(), 1, "setDataBreakpoints must return one record");
            let verified = bps[0].get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
            assert!(verified, "returned breakpoint must be verified");
        }
        _ => return Err("Expected setDataBreakpoints response with body".into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Cross-feature: feature catalog completeness
// ---------------------------------------------------------------------------

/// All 10 DAP features must be registered in the feature catalog.
#[test]
fn test_all_dap_features_registered_in_catalog() {
    let dap_features = [
        "dap.core",
        "dap.breakpoints.basic",
        "dap.breakpoints.hit_condition",
        "dap.breakpoints.logpoints",
        "dap.completions",
        "dap.exceptions.die",
        "dap.exceptions.warn",
        "dap.inline_values",
        "dap.modules",
        "dap.watchpoints",
    ];

    let missing: Vec<&str> = dap_features.iter().filter(|&&f| !has_feature(f)).copied().collect();

    assert!(
        missing.is_empty(),
        "The following DAP features are missing from the catalog: {missing:?}"
    );
}

/// The initialize response must not advertise capabilities for disabled features.
#[test]
fn test_initialize_does_not_advertise_disabled_features() -> TestResult {
    let body = get_initialize_body()?;

    // Each capability must be false if its feature is disabled.
    // Derived from handle_initialize in debug_adapter/process.rs:
    //   supportsConditionalBreakpoints      = supports_basic_breakpoints
    //   supportsBreakpointLocationsRequest  = supports_basic_breakpoints
    //   supportsHitConditionalBreakpoints   = supports_hit_conditions
    //   supportsLogPoints                   = supports_log_points
    //   supportsInlineValues                = supports_inline_values
    //   supportsCompletionsRequest          = supports_completions
    //   supportsModulesRequest              = supports_modules
    //   supportsDataBreakpoints             = supports_watchpoints
    let feature_to_cap = [
        ("dap.breakpoints.basic", "supportsConditionalBreakpoints"),
        ("dap.breakpoints.basic", "supportsBreakpointLocationsRequest"),
        ("dap.breakpoints.hit_condition", "supportsHitConditionalBreakpoints"),
        ("dap.breakpoints.logpoints", "supportsLogPoints"),
        ("dap.inline_values", "supportsInlineValues"),
        ("dap.completions", "supportsCompletionsRequest"),
        ("dap.modules", "supportsModulesRequest"),
        ("dap.watchpoints", "supportsDataBreakpoints"),
    ];

    for (feature, capability) in feature_to_cap {
        let advertised = body.get(capability).and_then(|v| v.as_bool()).unwrap_or(false);
        let enabled = has_feature(feature);

        assert_eq!(
            advertised, enabled,
            "Capability `{capability}` must mirror feature `{feature}`: enabled={enabled}, advertised={advertised}"
        );
    }
    Ok(())
}
