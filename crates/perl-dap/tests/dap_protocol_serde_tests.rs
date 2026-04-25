//! Protocol type serde compliance tests.
//!
//! Verifies that DAP protocol message types serialize/deserialize
//! correctly with proper camelCase field naming, optional field omission,
//! and round-trip fidelity for all request/response argument types.

use perl_dap::protocol::*;
use serde_json::json;

// ── Request / Response / Event base types ──────────────────────────

#[test]
fn request_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let req = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: Some(json!({"clientID": "vscode"})),
    };
    let json = serde_json::to_string(&req)?;
    let back: Request = serde_json::from_str(&json)?;
    assert_eq!(back.seq, 1);
    assert_eq!(back.command, "initialize");
    assert!(back.arguments.is_some());
    Ok(())
}

#[test]
fn request_without_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let req = Request {
        seq: 5,
        msg_type: "request".to_string(),
        command: "configurationDone".to_string(),
        arguments: None,
    };
    let json = serde_json::to_string(&req)?;
    assert!(!json.contains("arguments"), "None arguments should be omitted: {json}");
    let back: Request = serde_json::from_str(&json)?;
    assert!(back.arguments.is_none());
    Ok(())
}

#[test]
fn response_success_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let resp = Response {
        seq: 1,
        msg_type: "response".to_string(),
        request_seq: 1,
        success: true,
        command: "initialize".to_string(),
        message: None,
        body: Some(json!({"supportsConfigurationDoneRequest": true})),
    };
    let json = serde_json::to_string(&resp)?;
    assert!(json.contains("requestSeq"), "Should use camelCase: {json}");
    let back: Response = serde_json::from_str(&json)?;
    assert_eq!(back.request_seq, 1);
    assert!(back.success);
    assert!(back.message.is_none());
    Ok(())
}

#[test]
fn response_error_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let resp = Response {
        seq: 2,
        msg_type: "response".to_string(),
        request_seq: 2,
        success: false,
        command: "launch".to_string(),
        message: Some("Script not found".to_string()),
        body: None,
    };
    let json = serde_json::to_string(&resp)?;
    let back: Response = serde_json::from_str(&json)?;
    assert!(!back.success);
    assert_eq!(back.message.as_deref(), Some("Script not found"));
    assert!(back.body.is_none());
    Ok(())
}

#[test]
fn event_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let evt = Event {
        seq: 10,
        msg_type: "event".to_string(),
        event: "stopped".to_string(),
        body: Some(json!({"reason": "breakpoint", "threadId": 1})),
    };
    let json = serde_json::to_string(&evt)?;
    let back: Event = serde_json::from_str(&json)?;
    assert_eq!(back.event, "stopped");
    assert!(back.body.is_some());
    Ok(())
}

#[test]
fn event_without_body() -> Result<(), Box<dyn std::error::Error>> {
    let evt = Event {
        seq: 11,
        msg_type: "event".to_string(),
        event: "initialized".to_string(),
        body: None,
    };
    let json = serde_json::to_string(&evt)?;
    assert!(!json.contains("body"), "None body should be omitted: {json}");
    Ok(())
}

// ── Initialize types ───────────────────────────────────────────────

#[test]
fn initialize_request_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = InitializeRequestArguments {
        client_id: Some("vscode".to_string()),
        client_name: Some("Visual Studio Code".to_string()),
        adapter_id: "perl-rs".to_string(),
        locale: Some("en-US".to_string()),
        lines_start_at1: Some(true),
        columns_start_at1: Some(true),
        path_format: Some("path".to_string()),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("clientID"), "DAP spelling: {json}");
    assert!(json.contains("clientName"), "camelCase: {json}");
    assert!(json.contains("adapterID"), "DAP spelling: {json}");
    assert!(json.contains("linesStartAt1"), "camelCase: {json}");
    assert!(json.contains("columnsStartAt1"), "camelCase: {json}");
    assert!(json.contains("pathFormat"), "camelCase: {json}");

    let back: InitializeRequestArguments = serde_json::from_str(&json)?;
    assert_eq!(back.adapter_id, "perl-rs");
    assert_eq!(back.client_id.as_deref(), Some("vscode"));
    Ok(())
}

#[test]
fn initialize_request_args_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"adapterID": "perl-rs"}"#;
    let args: InitializeRequestArguments = serde_json::from_str(json)?;
    assert_eq!(args.adapter_id, "perl-rs");
    assert!(args.client_id.is_none());
    assert!(args.lines_start_at1.is_none());
    Ok(())
}

#[test]
fn initialize_request_args_accepts_legacy_id_spellings() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"clientId":"vscode","adapterId":"perl-rs"}"#;
    let args: InitializeRequestArguments = serde_json::from_str(json)?;
    assert_eq!(args.client_id.as_deref(), Some("vscode"));
    assert_eq!(args.adapter_id, "perl-rs");
    Ok(())
}

#[test]
fn capabilities_omits_none_fields() -> Result<(), Box<dyn std::error::Error>> {
    let caps = Capabilities {
        supports_configuration_done_request: Some(true),
        supports_evaluate_for_hovers: None,
        supports_conditional_breakpoints: None,
        supports_hit_conditional_breakpoints: None,
        supports_log_points: None,
        supports_exception_options: None,
        supports_exception_filter_options: None,
        supports_terminate_request: None,
        supports_inline_values: None,
        supports_function_breakpoints: None,
        supports_set_variable: None,
        supports_value_formatting_options: None,
        support_terminate_debuggee: None,
        supports_step_back: None,
        supports_data_breakpoints: None,
        exception_breakpoint_filters: None,
    };
    let json = serde_json::to_string(&caps)?;
    assert!(json.contains("supportsConfigurationDoneRequest"));
    assert!(!json.contains("supportsEvaluateForHovers"), "None should be omitted: {json}");
    assert!(!json.contains("exceptionBreakpointFilters"), "None should be omitted: {json}");
    Ok(())
}

// ── Launch / Attach arguments ──────────────────────────────────────

#[test]
fn launch_request_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = LaunchRequestArguments {
        program: "/workspace/script.pl".to_string(),
        args: Some(vec!["--verbose".to_string()]),
        cwd: Some("/workspace".to_string()),
        env: None,
        perl_path: Some("/usr/bin/perl".to_string()),
        stop_on_entry: Some(true),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("stopOnEntry"), "camelCase: {json}");
    assert!(json.contains("perlPath"), "camelCase: {json}");

    let back: LaunchRequestArguments = serde_json::from_str(&json)?;
    assert_eq!(back.program, "/workspace/script.pl");
    assert_eq!(back.stop_on_entry, Some(true));
    Ok(())
}

#[test]
fn launch_request_args_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"program": "test.pl"}"#;
    let args: LaunchRequestArguments = serde_json::from_str(json)?;
    assert_eq!(args.program, "test.pl");
    assert!(args.args.is_none());
    assert!(args.cwd.is_none());
    assert!(args.stop_on_entry.is_none());
    Ok(())
}

#[test]
fn attach_request_args_tcp_mode() -> Result<(), Box<dyn std::error::Error>> {
    let args = AttachRequestArguments {
        process_id: None,
        host: Some("localhost".to_string()),
        port: Some(13603),
        timeout: Some(5000),
        stop_on_entry: None,
    };
    let json = serde_json::to_string(&args)?;
    let back: AttachRequestArguments = serde_json::from_str(&json)?;
    assert_eq!(back.host.as_deref(), Some("localhost"));
    assert_eq!(back.port, Some(13603));
    assert_eq!(back.timeout, Some(5000));
    assert!(back.process_id.is_none());
    Ok(())
}

#[test]
fn attach_request_args_pid_mode() -> Result<(), Box<dyn std::error::Error>> {
    let args = AttachRequestArguments {
        process_id: Some(12345),
        host: None,
        port: None,
        timeout: None,
        stop_on_entry: None,
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("processId"), "camelCase: {json}");
    assert!(!json.contains("host"), "None host should be omitted: {json}");

    let back: AttachRequestArguments = serde_json::from_str(&json)?;
    assert_eq!(back.process_id, Some(12345));
    Ok(())
}

// ── Control flow arguments ─────────────────────────────────────────

#[test]
fn continue_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = ContinueArguments { thread_id: 1 };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("threadId"), "camelCase: {json}");
    let back: ContinueArguments = serde_json::from_str(&json)?;
    assert_eq!(back.thread_id, 1);
    Ok(())
}

#[test]
fn next_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = NextArguments { thread_id: 2 };
    let json = serde_json::to_string(&args)?;
    let back: NextArguments = serde_json::from_str(&json)?;
    assert_eq!(back.thread_id, 2);
    Ok(())
}

#[test]
fn step_in_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = StepInArguments { thread_id: 3 };
    let json = serde_json::to_string(&args)?;
    let back: StepInArguments = serde_json::from_str(&json)?;
    assert_eq!(back.thread_id, 3);
    Ok(())
}

#[test]
fn step_out_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = StepOutArguments { thread_id: 4 };
    let json = serde_json::to_string(&args)?;
    let back: StepOutArguments = serde_json::from_str(&json)?;
    assert_eq!(back.thread_id, 4);
    Ok(())
}

#[test]
fn pause_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = PauseArguments { thread_id: 5 };
    let json = serde_json::to_string(&args)?;
    let back: PauseArguments = serde_json::from_str(&json)?;
    assert_eq!(back.thread_id, 5);
    Ok(())
}

// ── Stack / Scope / Variable arguments ─────────────────────────────

#[test]
fn stack_trace_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = StackTraceArguments { thread_id: 1, start_frame: Some(0), levels: Some(20) };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("threadId"), "camelCase: {json}");
    assert!(json.contains("startFrame"), "camelCase: {json}");

    let back: StackTraceArguments = serde_json::from_str(&json)?;
    assert_eq!(back.thread_id, 1);
    assert_eq!(back.start_frame, Some(0));
    assert_eq!(back.levels, Some(20));
    Ok(())
}

#[test]
fn stack_trace_args_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"threadId": 1}"#;
    let args: StackTraceArguments = serde_json::from_str(json)?;
    assert_eq!(args.thread_id, 1);
    assert!(args.start_frame.is_none());
    assert!(args.levels.is_none());
    Ok(())
}

#[test]
fn scopes_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = ScopesArguments { frame_id: 42 };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("frameId"), "camelCase: {json}");
    let back: ScopesArguments = serde_json::from_str(&json)?;
    assert_eq!(back.frame_id, 42);
    Ok(())
}

#[test]
fn variables_args_full_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = VariablesArguments {
        variables_reference: 100,
        filter: Some("indexed".to_string()),
        start: Some(0),
        count: Some(50),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("variablesReference"), "camelCase: {json}");

    let back: VariablesArguments = serde_json::from_str(&json)?;
    assert_eq!(back.variables_reference, 100);
    assert_eq!(back.filter.as_deref(), Some("indexed"));
    assert_eq!(back.start, Some(0));
    assert_eq!(back.count, Some(50));
    Ok(())
}

#[test]
fn variables_args_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"variablesReference": 7}"#;
    let args: VariablesArguments = serde_json::from_str(json)?;
    assert_eq!(args.variables_reference, 7);
    assert!(args.filter.is_none());
    assert!(args.start.is_none());
    assert!(args.count.is_none());
    Ok(())
}

// ── Evaluate arguments ─────────────────────────────────────────────

#[test]
fn evaluate_args_hover_context() -> Result<(), Box<dyn std::error::Error>> {
    let args = EvaluateArguments {
        expression: "$x + 1".to_string(),
        frame_id: Some(0),
        context: Some("hover".to_string()),
        allow_side_effects: Some(false),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("allowSideEffects"), "camelCase: {json}");
    assert!(json.contains("frameId"), "camelCase: {json}");

    let back: EvaluateArguments = serde_json::from_str(&json)?;
    assert_eq!(back.expression, "$x + 1");
    assert_eq!(back.context.as_deref(), Some("hover"));
    assert_eq!(back.allow_side_effects, Some(false));
    Ok(())
}

#[test]
fn evaluate_args_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"expression": "$_"}"#;
    let args: EvaluateArguments = serde_json::from_str(json)?;
    assert_eq!(args.expression, "$_");
    assert!(args.frame_id.is_none());
    assert!(args.context.is_none());
    Ok(())
}

// ── Disconnect / Terminate arguments ───────────────────────────────

#[test]
fn disconnect_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = DisconnectArguments { restart: Some(false), terminate_debuggee: Some(true) };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("terminateDebuggee"), "camelCase: {json}");
    let back: DisconnectArguments = serde_json::from_str(&json)?;
    assert_eq!(back.terminate_debuggee, Some(true));
    assert_eq!(back.restart, Some(false));
    Ok(())
}

#[test]
fn terminate_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = TerminateArguments { restart: Some(true) };
    let json = serde_json::to_string(&args)?;
    let back: TerminateArguments = serde_json::from_str(&json)?;
    assert_eq!(back.restart, Some(true));
    Ok(())
}

// ── Breakpoint types ───────────────────────────────────────────────

#[test]
fn source_breakpoint_full_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let bp = SourceBreakpoint {
        line: 42,
        column: Some(10),
        condition: Some("$x > 5".to_string()),
        hit_condition: Some(">= 3".to_string()),
        log_message: Some("Value of x: {$x}".to_string()),
    };
    let json = serde_json::to_string(&bp)?;
    assert!(json.contains("hitCondition"), "camelCase: {json}");
    assert!(json.contains("logMessage"), "camelCase: {json}");

    let back: SourceBreakpoint = serde_json::from_str(&json)?;
    assert_eq!(back.line, 42);
    assert_eq!(back.column, Some(10));
    assert_eq!(back.condition.as_deref(), Some("$x > 5"));
    assert_eq!(back.hit_condition.as_deref(), Some(">= 3"));
    assert_eq!(back.log_message.as_deref(), Some("Value of x: {$x}"));
    Ok(())
}

#[test]
fn source_breakpoint_minimal() -> Result<(), Box<dyn std::error::Error>> {
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
fn function_breakpoint_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let bp = FunctionBreakpoint {
        name: "main::process_data".to_string(),
        condition: Some("$count > 100".to_string()),
        hit_condition: None,
    };
    let json = serde_json::to_string(&bp)?;
    let back: FunctionBreakpoint = serde_json::from_str(&json)?;
    assert_eq!(back.name, "main::process_data");
    assert_eq!(back.condition.as_deref(), Some("$count > 100"));
    assert!(back.hit_condition.is_none());
    Ok(())
}

#[test]
fn set_breakpoints_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetBreakpointsArguments {
        source: Source { path: Some("/ws/test.pl".to_string()), name: Some("test.pl".to_string()) },
        breakpoints: Some(vec![
            SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            },
            SourceBreakpoint {
                line: 20,
                column: None,
                condition: Some("$x".to_string()),
                hit_condition: None,
                log_message: None,
            },
        ]),
        source_modified: Some(false),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("sourceModified"), "camelCase: {json}");

    let back: SetBreakpointsArguments = serde_json::from_str(&json)?;
    let bps = back.breakpoints.as_ref().ok_or("Expected breakpoints")?;
    assert_eq!(bps.len(), 2);
    assert_eq!(bps[0].line, 10);
    assert_eq!(bps[1].condition.as_deref(), Some("$x"));
    Ok(())
}

// ── Exception breakpoints ──────────────────────────────────────────

#[test]
fn set_exception_breakpoints_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetExceptionBreakpointsArguments {
        filters: vec!["die".to_string()],
        filter_options: Some(vec![ExceptionFilterOption {
            filter_id: "die".to_string(),
            condition: Some("$_ =~ /fatal/".to_string()),
        }]),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("filterOptions"), "camelCase: {json}");
    assert!(json.contains("filterId"), "camelCase: {json}");

    let back: SetExceptionBreakpointsArguments = serde_json::from_str(&json)?;
    assert_eq!(back.filters, vec!["die"]);
    let opts = back.filter_options.as_ref().ok_or("Expected filter_options")?;
    assert_eq!(opts[0].filter_id, "die");
    Ok(())
}

#[test]
fn exception_breakpoint_filter_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let filter = ExceptionBreakpointFilter {
        filter: "die".to_string(),
        label: "Break on die/croak".to_string(),
        default: Some(false),
    };
    let json = serde_json::to_string(&filter)?;
    let back: ExceptionBreakpointFilter = serde_json::from_str(&json)?;
    assert_eq!(back.filter, "die");
    assert_eq!(back.label, "Break on die/croak");
    assert_eq!(back.default, Some(false));
    Ok(())
}

// ── Response body types ────────────────────────────────────────────

#[test]
fn threads_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ThreadsResponseBody {
        threads: vec![
            Thread { id: 1, name: "main".to_string() },
            Thread { id: 2, name: "worker".to_string() },
        ],
    };
    let json = serde_json::to_string(&body)?;
    let back: ThreadsResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.threads.len(), 2);
    assert_eq!(back.threads[0].name, "main");
    Ok(())
}

#[test]
fn continue_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ContinueResponseBody { all_threads_continued: true };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("allThreadsContinued"), "camelCase: {json}");
    let back: ContinueResponseBody = serde_json::from_str(&json)?;
    assert!(back.all_threads_continued);
    Ok(())
}

#[test]
fn evaluate_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = EvaluateResponseBody {
        result: "42".to_string(),
        type_: Some("SCALAR".to_string()),
        variables_reference: 0,
    };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("\"type\":"), "type field should use 'type' not 'type_': {json}");
    assert!(!json.contains("type_"), "type_ should not leak: {json}");

    let back: EvaluateResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.result, "42");
    assert_eq!(back.type_, Some("SCALAR".to_string()));
    Ok(())
}

#[test]
fn set_variable_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = SetVariableResponseBody {
        value: "new_value".to_string(),
        type_: Some("SCALAR".to_string()),
        variables_reference: 0,
    };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("variablesReference"), "camelCase: {json}");
    let back: SetVariableResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.value, "new_value");
    Ok(())
}

#[test]
fn scopes_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ScopesResponseBody {
        scopes: vec![
            Scope {
                name: "Locals".to_string(),
                presentation_hint: Some("locals".to_string()),
                variables_reference: 1,
                expensive: false,
            },
            Scope {
                name: "Globals".to_string(),
                presentation_hint: Some("globals".to_string()),
                variables_reference: 2,
                expensive: true,
            },
        ],
    };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("presentationHint"), "camelCase: {json}");
    assert!(json.contains("variablesReference"), "camelCase: {json}");
    let back: ScopesResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.scopes.len(), 2);
    assert!(!back.scopes[0].expensive);
    assert!(back.scopes[1].expensive);
    Ok(())
}

#[test]
fn stack_trace_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = StackTraceResponseBody {
        stack_frames: vec![ProtocolStackFrame {
            id: 0,
            name: "main::run".to_string(),
            source: Some(Source {
                path: Some("/a.pl".to_string()),
                name: Some("a.pl".to_string()),
            }),
            line: 10,
            column: 1,
            end_line: None,
            end_column: None,
        }],
        total_frames: Some(1),
    };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("stackFrames"), "camelCase: {json}");
    assert!(json.contains("totalFrames"), "camelCase: {json}");
    let back: StackTraceResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.stack_frames.len(), 1);
    assert_eq!(back.total_frames, Some(1));
    Ok(())
}

// ── Exception info ─────────────────────────────────────────────────

#[test]
fn exception_info_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ExceptionInfoResponseBody {
        exception_id: "die".to_string(),
        description: Some("Died at script.pl line 42".to_string()),
        break_mode: "always".to_string(),
        details: Some(ExceptionDetails {
            message: Some("Cannot open file".to_string()),
            type_name: Some("IO::Error".to_string()),
            stack_trace: Some(
                "  at main::open_file (script.pl:42)\n  at main (script.pl:10)".to_string(),
            ),
        }),
    };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("exceptionId"), "camelCase: {json}");
    assert!(json.contains("breakMode"), "camelCase: {json}");
    assert!(json.contains("typeName"), "camelCase: {json}");
    assert!(json.contains("stackTrace"), "camelCase: {json}");

    let back: ExceptionInfoResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.exception_id, "die");
    let details = back.details.as_ref().ok_or("Expected details")?;
    assert_eq!(details.type_name.as_deref(), Some("IO::Error"));
    Ok(())
}

#[test]
fn exception_info_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"exceptionId": "warn", "breakMode": "never"}"#;
    let body: ExceptionInfoResponseBody = serde_json::from_str(json)?;
    assert_eq!(body.exception_id, "warn");
    assert_eq!(body.break_mode, "never");
    assert!(body.description.is_none());
    assert!(body.details.is_none());
    Ok(())
}

// ── Goto / Source / StepInTargets ──────────────────────────────────

#[test]
fn goto_target_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let target = GotoTarget {
        id: 1,
        label: "line 42".to_string(),
        line: 42,
        column: Some(1),
        end_line: Some(42),
        end_column: Some(80),
    };
    let json = serde_json::to_string(&target)?;
    assert!(json.contains("endLine"), "camelCase: {json}");
    let back: GotoTarget = serde_json::from_str(&json)?;
    assert_eq!(back.id, 1);
    assert_eq!(back.line, 42);
    Ok(())
}

#[test]
fn step_in_target_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let target = StepInTarget { id: 5, label: "Foo::bar()".to_string() };
    let json = serde_json::to_string(&target)?;
    let back: StepInTarget = serde_json::from_str(&json)?;
    assert_eq!(back.id, 5);
    assert_eq!(back.label, "Foo::bar()");
    Ok(())
}

#[test]
fn source_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = SourceResponseBody {
        content: "#!/usr/bin/perl\nprint 'hello';\n".to_string(),
        mime_type: Some("text/x-perl".to_string()),
    };
    let json = serde_json::to_string(&body)?;
    assert!(json.contains("mimeType"), "camelCase: {json}");
    let back: SourceResponseBody = serde_json::from_str(&json)?;
    assert!(back.content.contains("print 'hello'"));
    assert_eq!(back.mime_type.as_deref(), Some("text/x-perl"));
    Ok(())
}

// ── SetVariable arguments ──────────────────────────────────────────

#[test]
fn set_variable_args_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetVariableArguments {
        variables_reference: 10,
        name: "$count".to_string(),
        value: "42".to_string(),
    };
    let json = serde_json::to_string(&args)?;
    assert!(json.contains("variablesReference"), "camelCase: {json}");
    let back: SetVariableArguments = serde_json::from_str(&json)?;
    assert_eq!(back.name, "$count");
    assert_eq!(back.value, "42");
    Ok(())
}

// ── BreakpointLocation types ───────────────────────────────────────

#[test]
fn breakpoint_location_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let loc =
        BreakpointLocation { line: 15, column: Some(1), end_line: Some(15), end_column: Some(40) };
    let json = serde_json::to_string(&loc)?;
    let back: BreakpointLocation = serde_json::from_str(&json)?;
    assert_eq!(back.line, 15);
    assert_eq!(back.column, Some(1));
    assert_eq!(back.end_line, Some(15));
    Ok(())
}

#[test]
fn breakpoint_locations_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = BreakpointLocationsResponseBody {
        breakpoints: vec![
            BreakpointLocation { line: 10, column: None, end_line: None, end_column: None },
            BreakpointLocation { line: 15, column: Some(5), end_line: None, end_column: None },
        ],
    };
    let json = serde_json::to_string(&body)?;
    let back: BreakpointLocationsResponseBody = serde_json::from_str(&json)?;
    assert_eq!(back.breakpoints.len(), 2);
    Ok(())
}
