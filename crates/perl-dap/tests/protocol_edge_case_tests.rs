//! Protocol edge-case tests for DAP message types
//!
//! Coverage target: crates/perl-dap/src/protocol.rs
//!
//! Tests:
//! - Malformed JSON deserialization
//! - Missing required fields
//! - Boundary / extreme values for seq numbers
//! - Very large string values
//! - Round-trip serialization of every top-level type
//! - Optional-field absence

use anyhow::Result;
use perl_dap::{
    AttachRequestArguments, Breakpoint, BreakpointLocation, BreakpointLocationsArguments,
    BreakpointLocationsResponseBody, CancelArguments, Capabilities, CompletionItem,
    CompletionsArguments, CompletionsResponseBody, ContinueArguments, ContinueResponseBody,
    DataBreakpoint, DataBreakpointInfoArguments, DataBreakpointInfoResponseBody,
    DisconnectArguments, EvaluateArguments, EvaluateResponseBody, Event, ExceptionBreakpointFilter,
    ExceptionDetails, ExceptionFilterOption, ExceptionInfoArguments, ExceptionInfoResponseBody,
    FunctionBreakpoint, GotoArguments, GotoTarget, GotoTargetsArguments, GotoTargetsResponseBody,
    InitializeRequestArguments, LaunchRequestArguments, LoadedSourcesResponseBody, Module,
    ModulesArguments, ModulesResponseBody, NextArguments, PauseArguments, ProtocolStackFrame,
    ProtocolVariable, Request, Response, RestartArguments, RestartFrameArguments, Scope,
    ScopesArguments, ScopesResponseBody, SetBreakpointsArguments, SetBreakpointsResponseBody,
    SetDataBreakpointsArguments, SetDataBreakpointsResponseBody, SetExceptionBreakpointsArguments,
    SetExpressionArguments, SetExpressionResponseBody, SetFunctionBreakpointsArguments,
    SetVariableArguments, SetVariableResponseBody, Source, SourceArguments, SourceBreakpoint,
    SourceResponseBody, StackTraceArguments, StackTraceResponseBody, StepInArguments, StepInTarget,
    StepInTargetsArguments, StepInTargetsResponseBody, StepOutArguments, TerminateArguments,
    TerminateThreadsArguments, Thread, ThreadsResponseBody, VariablesArguments,
    VariablesResponseBody,
};

// ============================================================================
// Malformed JSON
// ============================================================================

#[test]
fn test_malformed_json_empty_string() -> Result<()> {
    let result = serde_json::from_str::<Request>("");
    assert!(result.is_err(), "Empty string should not parse as Request");
    Ok(())
}

#[test]
fn test_malformed_json_bare_null() -> Result<()> {
    let result = serde_json::from_str::<Request>("null");
    assert!(result.is_err(), "null should not parse as Request");
    Ok(())
}

#[test]
fn test_malformed_json_truncated() -> Result<()> {
    let result = serde_json::from_str::<Request>(r#"{"seq": 1, "type": "req"#);
    assert!(result.is_err(), "Truncated JSON should fail");
    Ok(())
}

#[test]
fn test_malformed_json_array_instead_of_object() -> Result<()> {
    let result = serde_json::from_str::<Request>("[]");
    assert!(result.is_err(), "Array should not parse as Request");
    Ok(())
}

#[test]
fn test_malformed_json_numeric_instead_of_object() -> Result<()> {
    let result = serde_json::from_str::<Request>("42");
    assert!(result.is_err(), "Numeric value should not parse as Request");
    Ok(())
}

// ============================================================================
// Missing required fields
// ============================================================================

#[test]
fn test_request_missing_seq() -> Result<()> {
    let json = r#"{"type": "request", "command": "initialize"}"#;
    let result = serde_json::from_str::<Request>(json);
    assert!(result.is_err(), "Request without seq should fail");
    Ok(())
}

#[test]
fn test_request_missing_command() -> Result<()> {
    let json = r#"{"seq": 1, "type": "request"}"#;
    let result = serde_json::from_str::<Request>(json);
    assert!(result.is_err(), "Request without command should fail");
    Ok(())
}

#[test]
fn test_request_missing_type() -> Result<()> {
    let json = r#"{"seq": 1, "command": "initialize"}"#;
    let result = serde_json::from_str::<Request>(json);
    assert!(result.is_err(), "Request without type should fail");
    Ok(())
}

#[test]
fn test_response_missing_success() -> Result<()> {
    let json = r#"{"seq": 1, "type": "response", "requestSeq": 1, "command": "initialize"}"#;
    let result = serde_json::from_str::<Response>(json);
    assert!(result.is_err(), "Response without success should fail");
    Ok(())
}

#[test]
fn test_response_missing_request_seq() -> Result<()> {
    let json = r#"{"seq": 1, "type": "response", "success": true, "command": "initialize"}"#;
    let result = serde_json::from_str::<Response>(json);
    assert!(result.is_err(), "Response without requestSeq should fail");
    Ok(())
}

#[test]
fn test_event_missing_event_field() -> Result<()> {
    let json = r#"{"seq": 1, "type": "event"}"#;
    let result = serde_json::from_str::<Event>(json);
    assert!(result.is_err(), "Event without event field should fail");
    Ok(())
}

#[test]
fn test_initialize_args_missing_adapter_id() -> Result<()> {
    let json = r#"{"clientId": "vscode"}"#;
    let result = serde_json::from_str::<InitializeRequestArguments>(json);
    assert!(
        result.is_err(),
        "InitializeRequestArguments without adapterId should fail"
    );
    Ok(())
}

#[test]
fn test_launch_args_missing_program() -> Result<()> {
    let json = r#"{"args": ["--verbose"]}"#;
    let result = serde_json::from_str::<LaunchRequestArguments>(json);
    assert!(
        result.is_err(),
        "LaunchRequestArguments without program should fail"
    );
    Ok(())
}

#[test]
fn test_source_breakpoint_missing_line() -> Result<()> {
    let json = r#"{"column": 5}"#;
    let result = serde_json::from_str::<SourceBreakpoint>(json);
    assert!(result.is_err(), "SourceBreakpoint without line should fail");
    Ok(())
}

// ============================================================================
// Boundary / extreme values for seq numbers
// ============================================================================

#[test]
fn test_seq_zero() -> Result<()> {
    let json = r#"{"seq": 0, "type": "request", "command": "initialize"}"#;
    let req: Request = serde_json::from_str(json)?;
    assert_eq!(req.seq, 0);
    Ok(())
}

#[test]
fn test_seq_negative() -> Result<()> {
    let json = r#"{"seq": -1, "type": "request", "command": "initialize"}"#;
    let req: Request = serde_json::from_str(json)?;
    assert_eq!(req.seq, -1);
    Ok(())
}

#[test]
fn test_seq_i64_max() -> Result<()> {
    let json = format!(
        r#"{{"seq": {}, "type": "request", "command": "initialize"}}"#,
        i64::MAX
    );
    let req: Request = serde_json::from_str(&json)?;
    assert_eq!(req.seq, i64::MAX);
    Ok(())
}

#[test]
fn test_seq_i64_min() -> Result<()> {
    let json = format!(
        r#"{{"seq": {}, "type": "request", "command": "initialize"}}"#,
        i64::MIN
    );
    let req: Request = serde_json::from_str(&json)?;
    assert_eq!(req.seq, i64::MIN);
    Ok(())
}

#[test]
fn test_seq_overflow_beyond_i64() -> Result<()> {
    // 2^63 overflows i64
    let json = r#"{"seq": 9223372036854775808, "type": "request", "command": "initialize"}"#;
    let result = serde_json::from_str::<Request>(json);
    assert!(
        result.is_err(),
        "seq exceeding i64::MAX should fail to parse"
    );
    Ok(())
}

#[test]
fn test_seq_float_rejected() -> Result<()> {
    let json = r#"{"seq": 1.5, "type": "request", "command": "initialize"}"#;
    // serde_json may accept 1.0 as i64, but 1.5 should fail
    let result = serde_json::from_str::<Request>(json);
    assert!(result.is_err(), "Floating point seq should be rejected");
    Ok(())
}

// ============================================================================
// Very large string values
// ============================================================================

#[test]
fn test_large_command_string() -> Result<()> {
    let big_cmd = "x".repeat(100_000);
    let json = format!(
        r#"{{"seq": 1, "type": "request", "command": "{}"}}"#,
        big_cmd
    );
    let req: Request = serde_json::from_str(&json)?;
    assert_eq!(req.command.len(), 100_000);
    Ok(())
}

#[test]
fn test_large_expression_in_evaluate_args() -> Result<()> {
    let big_expr = "print ".to_string() + &"$x + ".repeat(10_000) + "$y";
    let args = EvaluateArguments {
        expression: big_expr.clone(),
        frame_id: None,
        context: None,
        allow_side_effects: None,
    };
    let json = serde_json::to_string(&args)?;
    let round_tripped: EvaluateArguments = serde_json::from_str(&json)?;
    assert_eq!(round_tripped.expression, big_expr);
    Ok(())
}

#[test]
fn test_large_error_message_in_response() -> Result<()> {
    let big_msg = "E".repeat(1_000_000);
    let resp = Response {
        seq: 1,
        msg_type: "response".to_string(),
        request_seq: 1,
        success: false,
        command: "evaluate".to_string(),
        message: Some(big_msg.clone()),
        body: None,
    };
    let json = serde_json::to_string(&resp)?;
    let rt: Response = serde_json::from_str(&json)?;
    assert_eq!(rt.message.as_deref(), Some(big_msg.as_str()));
    Ok(())
}

// ============================================================================
// Round-trip serialization of core message types
// ============================================================================

#[test]
fn test_request_round_trip() -> Result<()> {
    let req = Request {
        seq: 42,
        msg_type: "request".to_string(),
        command: "setBreakpoints".to_string(),
        arguments: Some(serde_json::json!({"source": {"path": "/tmp/test.pl"}})),
    };
    let json = serde_json::to_string(&req)?;
    let rt: Request = serde_json::from_str(&json)?;
    assert_eq!(rt.seq, 42);
    assert_eq!(rt.command, "setBreakpoints");
    assert!(rt.arguments.is_some());
    Ok(())
}

#[test]
fn test_request_round_trip_no_arguments() -> Result<()> {
    let req = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "threads".to_string(),
        arguments: None,
    };
    let json = serde_json::to_string(&req)?;
    // "arguments" should be absent (skip_serializing_if)
    assert!(
        !json.contains("arguments"),
        "arguments key should be omitted when None"
    );
    let rt: Request = serde_json::from_str(&json)?;
    assert!(rt.arguments.is_none());
    Ok(())
}

#[test]
fn test_response_success_round_trip() -> Result<()> {
    let resp = Response {
        seq: 2,
        msg_type: "response".to_string(),
        request_seq: 1,
        success: true,
        command: "initialize".to_string(),
        message: None,
        body: Some(serde_json::json!({"supportsConfigurationDoneRequest": true})),
    };
    let json = serde_json::to_string(&resp)?;
    let rt: Response = serde_json::from_str(&json)?;
    assert!(rt.success);
    assert!(rt.body.is_some());
    assert!(rt.message.is_none());
    Ok(())
}

#[test]
fn test_response_error_round_trip() -> Result<()> {
    let resp = Response {
        seq: 3,
        msg_type: "response".to_string(),
        request_seq: 2,
        success: false,
        command: "evaluate".to_string(),
        message: Some("Evaluation failed".to_string()),
        body: None,
    };
    let json = serde_json::to_string(&resp)?;
    let rt: Response = serde_json::from_str(&json)?;
    assert!(!rt.success);
    assert_eq!(rt.message.as_deref(), Some("Evaluation failed"));
    Ok(())
}

#[test]
fn test_event_round_trip() -> Result<()> {
    let evt = Event {
        seq: 10,
        msg_type: "event".to_string(),
        event: "stopped".to_string(),
        body: Some(serde_json::json!({"reason": "breakpoint", "threadId": 1})),
    };
    let json = serde_json::to_string(&evt)?;
    let rt: Event = serde_json::from_str(&json)?;
    assert_eq!(rt.event, "stopped");
    assert!(rt.body.is_some());
    Ok(())
}

#[test]
fn test_event_no_body_round_trip() -> Result<()> {
    let evt = Event {
        seq: 11,
        msg_type: "event".to_string(),
        event: "initialized".to_string(),
        body: None,
    };
    let json = serde_json::to_string(&evt)?;
    assert!(!json.contains("body"));
    let rt: Event = serde_json::from_str(&json)?;
    assert!(rt.body.is_none());
    Ok(())
}

// ============================================================================
// Breakpoint types
// ============================================================================

#[test]
fn test_source_breakpoint_minimal() -> Result<()> {
    let json = r#"{"line": 10}"#;
    let bp: SourceBreakpoint = serde_json::from_str(json)?;
    assert_eq!(bp.line, 10);
    assert!(bp.column.is_none());
    assert!(bp.condition.is_none());
    assert!(bp.hit_condition.is_none());
    assert!(bp.log_message.is_none());
    Ok(())
}

#[test]
fn test_source_breakpoint_full() -> Result<()> {
    let bp = SourceBreakpoint {
        line: 42,
        column: Some(5),
        condition: Some("$x > 10".to_string()),
        hit_condition: Some(">= 3".to_string()),
        log_message: Some("hit line 42".to_string()),
    };
    let json = serde_json::to_string(&bp)?;
    let rt: SourceBreakpoint = serde_json::from_str(&json)?;
    assert_eq!(rt.line, 42);
    assert_eq!(rt.column, Some(5));
    assert_eq!(rt.condition.as_deref(), Some("$x > 10"));
    assert_eq!(rt.hit_condition.as_deref(), Some(">= 3"));
    assert_eq!(rt.log_message.as_deref(), Some("hit line 42"));
    Ok(())
}

#[test]
fn test_breakpoint_verified_round_trip() -> Result<()> {
    let bp = Breakpoint {
        id: 1,
        verified: true,
        line: 10,
        column: None,
        message: None,
    };
    let json = serde_json::to_string(&bp)?;
    let rt: Breakpoint = serde_json::from_str(&json)?;
    assert_eq!(rt.id, 1);
    assert!(rt.verified);
    assert_eq!(rt.line, 10);
    Ok(())
}

#[test]
fn test_breakpoint_unverified_with_message() -> Result<()> {
    let bp = Breakpoint {
        id: 2,
        verified: false,
        line: 5,
        column: Some(1),
        message: Some("Line is not executable".to_string()),
    };
    let json = serde_json::to_string(&bp)?;
    let rt: Breakpoint = serde_json::from_str(&json)?;
    assert!(!rt.verified);
    assert_eq!(rt.message.as_deref(), Some("Line is not executable"));
    Ok(())
}

#[test]
fn test_set_breakpoints_args_round_trip() -> Result<()> {
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some("/tmp/test.pl".to_string()),
            name: None,
        },
        breakpoints: Some(vec![
            SourceBreakpoint {
                line: 1,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            },
            SourceBreakpoint {
                line: 10,
                column: Some(3),
                condition: Some("1".to_string()),
                hit_condition: None,
                log_message: None,
            },
        ]),
        source_modified: Some(false),
    };
    let json = serde_json::to_string(&args)?;
    let rt: SetBreakpointsArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.source.path.as_deref(), Some("/tmp/test.pl"));
    let bps = rt.breakpoints.as_ref().map_or(0, |v| v.len());
    assert_eq!(bps, 2);
    Ok(())
}

#[test]
fn test_set_breakpoints_response_body_round_trip() -> Result<()> {
    let body = SetBreakpointsResponseBody {
        breakpoints: vec![Breakpoint {
            id: 1,
            verified: true,
            line: 1,
            column: None,
            message: None,
        }],
    };
    let json = serde_json::to_string(&body)?;
    let rt: SetBreakpointsResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.breakpoints.len(), 1);
    Ok(())
}

// ============================================================================
// Initialize / Capabilities
// ============================================================================

#[test]
fn test_initialize_args_minimal() -> Result<()> {
    let json = r#"{"adapterId": "perl-rs"}"#;
    let args: InitializeRequestArguments = serde_json::from_str(json)?;
    assert_eq!(args.adapter_id, "perl-rs");
    assert!(args.client_id.is_none());
    assert!(args.lines_start_at1.is_none());
    Ok(())
}

#[test]
fn test_capabilities_round_trip() -> Result<()> {
    let caps = Capabilities {
        supports_configuration_done_request: Some(true),
        supports_evaluate_for_hovers: Some(true),
        supports_conditional_breakpoints: Some(true),
        supports_hit_conditional_breakpoints: Some(false),
        supports_log_points: Some(true),
        supports_exception_options: None,
        supports_exception_filter_options: None,
        supports_terminate_request: Some(true),
        supports_inline_values: Some(true),
        supports_function_breakpoints: Some(true),
        supports_set_variable: Some(true),
        supports_value_formatting_options: None,
        support_terminate_debuggee: Some(true),
        supports_step_back: Some(false),
        supports_data_breakpoints: Some(false),
        exception_breakpoint_filters: Some(vec![ExceptionBreakpointFilter {
            filter: "all".to_string(),
            label: "All Exceptions".to_string(),
            default: Some(false),
        }]),
    };
    let json = serde_json::to_string(&caps)?;
    let rt: Capabilities = serde_json::from_str(&json)?;
    assert_eq!(rt.supports_configuration_done_request, Some(true));
    assert_eq!(rt.supports_step_back, Some(false));
    let filters = rt.exception_breakpoint_filters.as_ref();
    assert_eq!(filters.map(|f| f.len()), Some(1));
    Ok(())
}

// ============================================================================
// Launch / Attach args
// ============================================================================

#[test]
fn test_launch_args_round_trip() -> Result<()> {
    let args = LaunchRequestArguments {
        program: "/workspace/script.pl".to_string(),
        args: Some(vec!["--verbose".to_string()]),
        cwd: Some("/workspace".to_string()),
        env: Some(std::collections::HashMap::from([(
            "PERL5LIB".to_string(),
            "lib".to_string(),
        )])),
        perl_path: Some("/usr/bin/perl".to_string()),
        stop_on_entry: Some(true),
    };
    let json = serde_json::to_string(&args)?;
    let rt: LaunchRequestArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.program, "/workspace/script.pl");
    assert_eq!(rt.stop_on_entry, Some(true));
    Ok(())
}

#[test]
fn test_attach_args_round_trip() -> Result<()> {
    let args = AttachRequestArguments {
        process_id: None,
        host: Some("127.0.0.1".to_string()),
        port: Some(13603),
        timeout: Some(5000),
        stop_on_entry: None,
    };
    let json = serde_json::to_string(&args)?;
    let rt: AttachRequestArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.host.as_deref(), Some("127.0.0.1"));
    assert_eq!(rt.port, Some(13603));
    Ok(())
}

// ============================================================================
// Stack / Scopes / Variables
// ============================================================================

#[test]
fn test_stack_trace_args_round_trip() -> Result<()> {
    let args = StackTraceArguments {
        thread_id: 1,
        start_frame: Some(0),
        levels: Some(20),
    };
    let json = serde_json::to_string(&args)?;
    let rt: StackTraceArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.thread_id, 1);
    Ok(())
}

#[test]
fn test_stack_trace_response_body() -> Result<()> {
    let body = StackTraceResponseBody {
        stack_frames: vec![ProtocolStackFrame {
            id: 0,
            name: "main".to_string(),
            source: Some(Source {
                path: Some("/tmp/t.pl".to_string()),
                name: None,
            }),
            line: 1,
            column: 0,
            end_line: None,
            end_column: None,
        }],
        total_frames: Some(1),
    };
    let json = serde_json::to_string(&body)?;
    let rt: StackTraceResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.stack_frames.len(), 1);
    assert_eq!(rt.stack_frames[0].name, "main");
    Ok(())
}

#[test]
fn test_scopes_round_trip() -> Result<()> {
    let body = ScopesResponseBody {
        scopes: vec![Scope {
            name: "Locals".to_string(),
            presentation_hint: Some("locals".to_string()),
            variables_reference: 100,
            expensive: false,
        }],
    };
    let json = serde_json::to_string(&body)?;
    let rt: ScopesResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.scopes[0].variables_reference, 100);
    Ok(())
}

#[test]
fn test_variables_round_trip() -> Result<()> {
    let body = VariablesResponseBody {
        variables: vec![ProtocolVariable {
            name: "$x".to_string(),
            value: "42".to_string(),
            type_: Some("SCALAR".to_string()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        }],
    };
    let json = serde_json::to_string(&body)?;
    let rt: VariablesResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.variables[0].name, "$x");
    assert_eq!(rt.variables[0].value, "42");
    Ok(())
}

// ============================================================================
// Control flow arguments
// ============================================================================

#[test]
fn test_continue_args_round_trip() -> Result<()> {
    let args = ContinueArguments { thread_id: 1 };
    let json = serde_json::to_string(&args)?;
    let rt: ContinueArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.thread_id, 1);
    Ok(())
}

#[test]
fn test_continue_response_body() -> Result<()> {
    let body = ContinueResponseBody {
        all_threads_continued: true,
    };
    let json = serde_json::to_string(&body)?;
    let rt: ContinueResponseBody = serde_json::from_str(&json)?;
    assert!(rt.all_threads_continued);
    Ok(())
}

#[test]
fn test_next_step_pause_args() -> Result<()> {
    // All share the same shape (thread_id only)
    for (name, json_str) in [
        (
            "next",
            serde_json::to_string(&NextArguments { thread_id: 1 })?,
        ),
        (
            "stepIn",
            serde_json::to_string(&StepInArguments { thread_id: 2 })?,
        ),
        (
            "stepOut",
            serde_json::to_string(&StepOutArguments { thread_id: 3 })?,
        ),
        (
            "pause",
            serde_json::to_string(&PauseArguments { thread_id: 4 })?,
        ),
    ] {
        assert!(
            json_str.contains("threadId"),
            "{name} serialization should contain threadId"
        );
    }
    Ok(())
}

// ============================================================================
// Evaluate
// ============================================================================

#[test]
fn test_evaluate_args_minimal() -> Result<()> {
    let json = r#"{"expression": "$x"}"#;
    let args: EvaluateArguments = serde_json::from_str(json)?;
    assert_eq!(args.expression, "$x");
    assert!(args.frame_id.is_none());
    assert!(args.context.is_none());
    Ok(())
}

#[test]
fn test_evaluate_response_body_round_trip() -> Result<()> {
    let body = EvaluateResponseBody {
        result: "42".to_string(),
        type_: Some("SCALAR".to_string()),
        variables_reference: 0,
    };
    let json = serde_json::to_string(&body)?;
    let rt: EvaluateResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.result, "42");
    Ok(())
}

// ============================================================================
// Disconnect / Terminate
// ============================================================================

#[test]
fn test_disconnect_args_round_trip() -> Result<()> {
    let args = DisconnectArguments {
        restart: Some(false),
        terminate_debuggee: Some(true),
    };
    let json = serde_json::to_string(&args)?;
    let rt: DisconnectArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.terminate_debuggee, Some(true));
    Ok(())
}

#[test]
fn test_terminate_args_round_trip() -> Result<()> {
    let args = TerminateArguments { restart: None };
    let json = serde_json::to_string(&args)?;
    assert!(!json.contains("restart"), "None restart should be omitted");
    Ok(())
}

// ============================================================================
// Function breakpoints / Exception breakpoints
// ============================================================================

#[test]
fn test_function_breakpoint_round_trip() -> Result<()> {
    let args = SetFunctionBreakpointsArguments {
        breakpoints: vec![FunctionBreakpoint {
            name: "main".to_string(),
            condition: Some("1".to_string()),
            hit_condition: None,
        }],
    };
    let json = serde_json::to_string(&args)?;
    let rt: SetFunctionBreakpointsArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.breakpoints[0].name, "main");
    Ok(())
}

#[test]
fn test_exception_breakpoints_round_trip() -> Result<()> {
    let args = SetExceptionBreakpointsArguments {
        filters: vec!["all".to_string(), "uncaught".to_string()],
        filter_options: Some(vec![ExceptionFilterOption {
            filter_id: "all".to_string(),
            condition: None,
        }]),
    };
    let json = serde_json::to_string(&args)?;
    let rt: SetExceptionBreakpointsArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.filters.len(), 2);
    Ok(())
}

// ============================================================================
// Thread / Scopes args
// ============================================================================

#[test]
fn test_threads_response_body() -> Result<()> {
    let body = ThreadsResponseBody {
        threads: vec![Thread {
            id: 1,
            name: "main".to_string(),
        }],
    };
    let json = serde_json::to_string(&body)?;
    let rt: ThreadsResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.threads[0].id, 1);
    Ok(())
}

#[test]
fn test_scopes_args() -> Result<()> {
    let args = ScopesArguments { frame_id: 0 };
    let json = serde_json::to_string(&args)?;
    let rt: ScopesArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.frame_id, 0);
    Ok(())
}

#[test]
fn test_variables_args() -> Result<()> {
    let args = VariablesArguments {
        variables_reference: 100,
        filter: Some("named".to_string()),
        start: Some(0),
        count: Some(50),
    };
    let json = serde_json::to_string(&args)?;
    let rt: VariablesArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.variables_reference, 100);
    assert_eq!(rt.filter.as_deref(), Some("named"));
    Ok(())
}

// ============================================================================
// SetVariable
// ============================================================================

#[test]
fn test_set_variable_args_round_trip() -> Result<()> {
    let args = SetVariableArguments {
        variables_reference: 10,
        name: "$x".to_string(),
        value: "99".to_string(),
    };
    let json = serde_json::to_string(&args)?;
    let rt: SetVariableArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.name, "$x");
    assert_eq!(rt.value, "99");
    Ok(())
}

#[test]
fn test_set_variable_response_body() -> Result<()> {
    let body = SetVariableResponseBody {
        value: "99".to_string(),
        type_: Some("SCALAR".to_string()),
        variables_reference: 0,
    };
    let json = serde_json::to_string(&body)?;
    let rt: SetVariableResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.value, "99");
    Ok(())
}

// ============================================================================
// Breakpoint Locations
// ============================================================================

#[test]
fn test_breakpoint_locations_args() -> Result<()> {
    let args = BreakpointLocationsArguments {
        source: Source {
            path: Some("/tmp/t.pl".to_string()),
            name: None,
        },
        line: 1,
        end_line: Some(10),
    };
    let json = serde_json::to_string(&args)?;
    let rt: BreakpointLocationsArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.line, 1);
    assert_eq!(rt.end_line, Some(10));
    Ok(())
}

#[test]
fn test_breakpoint_locations_response_body() -> Result<()> {
    let body = BreakpointLocationsResponseBody {
        breakpoints: vec![BreakpointLocation {
            line: 3,
            column: None,
            end_line: None,
            end_column: None,
        }],
    };
    let json = serde_json::to_string(&body)?;
    let rt: BreakpointLocationsResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.breakpoints.len(), 1);
    assert_eq!(rt.breakpoints[0].line, 3);
    Ok(())
}

// ============================================================================
// Source request
// ============================================================================

#[test]
fn test_source_args_round_trip() -> Result<()> {
    let args = SourceArguments {
        source_reference: Some(1),
        source: Some(Source {
            path: Some("/tmp/t.pl".to_string()),
            name: None,
        }),
    };
    let json = serde_json::to_string(&args)?;
    let rt: SourceArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.source_reference, Some(1));
    Ok(())
}

#[test]
fn test_source_response_body_round_trip() -> Result<()> {
    let body = SourceResponseBody {
        content: "use strict;\nprint 'hello';\n".to_string(),
        mime_type: Some("text/x-perl".to_string()),
    };
    let json = serde_json::to_string(&body)?;
    let rt: SourceResponseBody = serde_json::from_str(&json)?;
    assert!(rt.content.contains("use strict"));
    Ok(())
}

// ============================================================================
// StepInTargets / GotoTargets
// ============================================================================

#[test]
fn test_step_in_targets_round_trip() -> Result<()> {
    let args = StepInTargetsArguments { frame_id: 0 };
    let body = StepInTargetsResponseBody {
        targets: vec![StepInTarget {
            id: 1,
            label: "foo()".to_string(),
        }],
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: StepInTargetsArguments = serde_json::from_str(&json_a)?;
    let rt: StepInTargetsResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.targets[0].label, "foo()");
    Ok(())
}

#[test]
fn test_goto_targets_round_trip() -> Result<()> {
    let args = GotoTargetsArguments {
        source: Source {
            path: Some("/tmp/t.pl".to_string()),
            name: None,
        },
        line: 5,
        column: None,
    };
    let body = GotoTargetsResponseBody {
        targets: vec![GotoTarget {
            id: 1,
            label: "line 5".to_string(),
            line: 5,
            column: None,
            end_line: None,
            end_column: None,
        }],
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: GotoTargetsArguments = serde_json::from_str(&json_a)?;
    let rt: GotoTargetsResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.targets[0].line, 5);
    Ok(())
}

// ============================================================================
// Exception info
// ============================================================================

#[test]
fn test_exception_info_round_trip() -> Result<()> {
    let args = ExceptionInfoArguments { thread_id: 1 };
    let body = ExceptionInfoResponseBody {
        exception_id: "die".to_string(),
        description: Some("Died at script.pl line 10".to_string()),
        break_mode: "always".to_string(),
        details: Some(ExceptionDetails {
            message: Some("something went wrong".to_string()),
            type_name: Some("die".to_string()),
            stack_trace: Some("at script.pl line 10\n".to_string()),
        }),
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: ExceptionInfoArguments = serde_json::from_str(&json_a)?;
    let rt: ExceptionInfoResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.exception_id, "die");
    assert_eq!(rt.break_mode, "always");
    Ok(())
}

// ============================================================================
// SetExpression
// ============================================================================

#[test]
fn test_set_expression_round_trip() -> Result<()> {
    let args = SetExpressionArguments {
        expression: "$x".to_string(),
        value: "42".to_string(),
        frame_id: Some(0),
    };
    let body = SetExpressionResponseBody {
        value: "42".to_string(),
        type_: Some("SCALAR".to_string()),
        variables_reference: 0,
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let rt_a: SetExpressionArguments = serde_json::from_str(&json_a)?;
    let rt_b: SetExpressionResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt_a.expression, "$x");
    assert_eq!(rt_b.value, "42");
    Ok(())
}

// ============================================================================
// Restart / RestartFrame / TerminateThreads
// ============================================================================

#[test]
fn test_restart_args_round_trip() -> Result<()> {
    let args = RestartArguments {
        arguments: Some(serde_json::json!({"program": "t.pl"})),
    };
    let json = serde_json::to_string(&args)?;
    let rt: RestartArguments = serde_json::from_str(&json)?;
    assert!(rt.arguments.is_some());
    Ok(())
}

#[test]
fn test_restart_frame_args() -> Result<()> {
    let args = RestartFrameArguments { frame_id: 2 };
    let json = serde_json::to_string(&args)?;
    let rt: RestartFrameArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.frame_id, 2);
    Ok(())
}

#[test]
fn test_terminate_threads_args() -> Result<()> {
    let args = TerminateThreadsArguments {
        thread_ids: Some(vec![1, 2, 3]),
    };
    let json = serde_json::to_string(&args)?;
    let rt: TerminateThreadsArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.thread_ids.as_ref().map(|v| v.len()), Some(3));
    Ok(())
}

// ============================================================================
// Goto / Cancel
// ============================================================================

#[test]
fn test_goto_args() -> Result<()> {
    let args = GotoArguments {
        thread_id: 1,
        target_id: 5,
    };
    let json = serde_json::to_string(&args)?;
    let rt: GotoArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.thread_id, 1);
    assert_eq!(rt.target_id, 5);
    Ok(())
}

#[test]
fn test_cancel_args() -> Result<()> {
    let args = CancelArguments {
        request_id: Some(42),
        progress_id: None,
    };
    let json = serde_json::to_string(&args)?;
    let rt: CancelArguments = serde_json::from_str(&json)?;
    assert_eq!(rt.request_id, Some(42));
    assert!(rt.progress_id.is_none());
    Ok(())
}

// ============================================================================
// Loaded Sources / Modules
// ============================================================================

#[test]
fn test_loaded_sources_response() -> Result<()> {
    let body = LoadedSourcesResponseBody {
        sources: vec![Source {
            path: Some("/tmp/a.pl".to_string()),
            name: Some("a.pl".to_string()),
        }],
    };
    let json = serde_json::to_string(&body)?;
    let rt: LoadedSourcesResponseBody = serde_json::from_str(&json)?;
    assert_eq!(rt.sources.len(), 1);
    Ok(())
}

#[test]
fn test_modules_round_trip() -> Result<()> {
    let args = ModulesArguments {
        start_module: Some(0),
        module_count: Some(10),
    };
    let body = ModulesResponseBody {
        modules: vec![Module {
            id: "Foo::Bar".to_string(),
            name: "Foo::Bar".to_string(),
            path: Some("/lib/Foo/Bar.pm".to_string()),
        }],
        total_modules: Some(1),
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: ModulesArguments = serde_json::from_str(&json_a)?;
    let rt: ModulesResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.modules[0].name, "Foo::Bar");
    Ok(())
}

// ============================================================================
// Completions
// ============================================================================

#[test]
fn test_completions_round_trip() -> Result<()> {
    let args = CompletionsArguments {
        text: "$x->".to_string(),
        column: 4,
        frame_id: Some(0),
        line: None,
    };
    let body = CompletionsResponseBody {
        targets: vec![CompletionItem {
            label: "method".to_string(),
            type_: Some("method".to_string()),
            text: None,
            sort_text: None,
            detail: Some("sub method".to_string()),
            start: None,
            length: None,
        }],
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: CompletionsArguments = serde_json::from_str(&json_a)?;
    let rt: CompletionsResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.targets[0].label, "method");
    Ok(())
}

// ============================================================================
// Data Breakpoints
// ============================================================================

#[test]
fn test_data_breakpoint_info_round_trip() -> Result<()> {
    let args = DataBreakpointInfoArguments {
        name: "$x".to_string(),
        variables_reference: Some(10),
        frame_id: None,
    };
    let body = DataBreakpointInfoResponseBody {
        data_id: Some("var_x_1".to_string()),
        description: "Write access to $x".to_string(),
        access_types: Some(vec!["write".to_string(), "readWrite".to_string()]),
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: DataBreakpointInfoArguments = serde_json::from_str(&json_a)?;
    let rt: DataBreakpointInfoResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.data_id.as_deref(), Some("var_x_1"));
    Ok(())
}

#[test]
fn test_set_data_breakpoints_round_trip() -> Result<()> {
    let args = SetDataBreakpointsArguments {
        breakpoints: vec![DataBreakpoint {
            data_id: "var_x_1".to_string(),
            access_type: Some("write".to_string()),
            condition: None,
            hit_condition: None,
        }],
    };
    let body = SetDataBreakpointsResponseBody {
        breakpoints: vec![Breakpoint {
            id: 1,
            verified: true,
            line: 10,
            column: None,
            message: None,
        }],
    };
    let json_a = serde_json::to_string(&args)?;
    let json_b = serde_json::to_string(&body)?;
    let _: SetDataBreakpointsArguments = serde_json::from_str(&json_a)?;
    let rt: SetDataBreakpointsResponseBody = serde_json::from_str(&json_b)?;
    assert_eq!(rt.breakpoints.len(), 1);
    Ok(())
}

// ============================================================================
// Unicode edge cases
// ============================================================================

#[test]
fn test_unicode_in_command() -> Result<()> {
    let req = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "eval_\u{1F600}".to_string(),
        arguments: None,
    };
    let json = serde_json::to_string(&req)?;
    let rt: Request = serde_json::from_str(&json)?;
    assert!(rt.command.contains('\u{1F600}'));
    Ok(())
}

#[test]
fn test_unicode_in_evaluate_expression() -> Result<()> {
    let args = EvaluateArguments {
        expression: "print \"\u{00E9}\u{00E8}\u{00EA}\";".to_string(),
        frame_id: None,
        context: None,
        allow_side_effects: None,
    };
    let json = serde_json::to_string(&args)?;
    let rt: EvaluateArguments = serde_json::from_str(&json)?;
    assert!(rt.expression.contains('\u{00E9}'));
    Ok(())
}

// ============================================================================
// Extra unknown fields (forward compatibility)
// ============================================================================

#[test]
fn test_request_with_extra_fields() -> Result<()> {
    // serde by default ignores unknown fields (unless deny_unknown_fields is set)
    let json = r#"{"seq": 1, "type": "request", "command": "init", "futureField": true}"#;
    let req: Request = serde_json::from_str(json)?;
    assert_eq!(req.command, "init");
    Ok(())
}

#[test]
fn test_source_breakpoint_with_extra_fields() -> Result<()> {
    let json = r#"{"line": 5, "unknownField": "value"}"#;
    let bp: SourceBreakpoint = serde_json::from_str(json)?;
    assert_eq!(bp.line, 5);
    Ok(())
}
