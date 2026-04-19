//! Coverage audit tests for the perl-dap crate
//!
//! These tests target specific coverage gaps identified during an audit of
//! the DAP crate test suite. They focus on:
//!
//! - Protocol type serde round-trips (AC5)
//! - DapMessage serialization/deserialization
//! - DapServer/DapConfig construction and mode handling
//! - BreakpointStore edge cases (is_empty, hit outcomes, edit adjustments)
//! - BreakpointRecord::to_protocol fidelity
//! - Inline value edge cases
//! - TcpAttachConfig builder ergonomics
//! - Feature catalog runtime queries

use perl_dap::breakpoints::{BreakpointRecord, BreakpointStore};
use perl_dap::protocol::*;
use perl_dap::{
    AttachConfiguration, DapConfig, DapMessage, DapMode, DapServer, DebugAdapter,
    LaunchConfiguration,
};
use serde_json::json;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

// ============================================================================
// Protocol type serde round-trip tests (AC5)
// ============================================================================

#[test]
fn test_request_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request {
        seq: 1,
        msg_type: "request".to_string(),
        command: "initialize".to_string(),
        arguments: Some(json!({"clientId": "vscode", "adapterId": "perl-rs"})),
    };

    let json = serde_json::to_string(&request)?;
    let deserialized: Request = serde_json::from_str(&json)?;

    assert_eq!(deserialized.seq, 1);
    assert_eq!(deserialized.msg_type, "request");
    assert_eq!(deserialized.command, "initialize");
    assert!(deserialized.arguments.is_some());
    Ok(())
}

#[test]
fn test_request_without_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request {
        seq: 2,
        msg_type: "request".to_string(),
        command: "configurationDone".to_string(),
        arguments: None,
    };

    let json = serde_json::to_string(&request)?;
    // arguments should be omitted from JSON
    assert!(
        !json.contains("arguments"),
        "None arguments should be skipped in serialization"
    );

    let deserialized: Request = serde_json::from_str(&json)?;
    assert!(deserialized.arguments.is_none());
    Ok(())
}

#[test]
fn test_response_success_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let response = Response {
        seq: 1,
        msg_type: "response".to_string(),
        request_seq: 1,
        success: true,
        command: "initialize".to_string(),
        message: None,
        body: Some(json!({"supportsConfigurationDoneRequest": true})),
    };

    let json = serde_json::to_string(&response)?;
    let deserialized: Response = serde_json::from_str(&json)?;

    assert!(deserialized.success);
    assert!(deserialized.message.is_none());
    assert!(deserialized.body.is_some());
    Ok(())
}

#[test]
fn test_response_error_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let response = Response {
        seq: 2,
        msg_type: "response".to_string(),
        request_seq: 1,
        success: false,
        command: "unknownCommand".to_string(),
        message: Some("Unknown command: unknownCommand".to_string()),
        body: None,
    };

    let json = serde_json::to_string(&response)?;
    let deserialized: Response = serde_json::from_str(&json)?;

    assert!(!deserialized.success);
    assert!(
        deserialized
            .message
            .as_ref()
            .is_some_and(|m| m.contains("Unknown command"))
    );
    assert!(deserialized.body.is_none());
    Ok(())
}

#[test]
fn test_event_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let event = Event {
        seq: 1,
        msg_type: "event".to_string(),
        event: "stopped".to_string(),
        body: Some(json!({"reason": "breakpoint", "threadId": 1})),
    };

    let json = serde_json::to_string(&event)?;
    let deserialized: Event = serde_json::from_str(&json)?;

    assert_eq!(deserialized.event, "stopped");
    assert!(deserialized.body.is_some());
    Ok(())
}

#[test]
fn test_event_without_body() -> Result<(), Box<dyn std::error::Error>> {
    let event = Event {
        seq: 1,
        msg_type: "event".to_string(),
        event: "initialized".to_string(),
        body: None,
    };

    let json = serde_json::to_string(&event)?;
    assert!(!json.contains("body"), "None body should be skipped");
    Ok(())
}

#[test]
fn test_source_breakpoint_full_fields() -> Result<(), Box<dyn std::error::Error>> {
    let bp = SourceBreakpoint {
        line: 42,
        column: Some(5),
        condition: Some("$x > 10".to_string()),
        hit_condition: Some(">= 3".to_string()),
        log_message: Some("hit breakpoint at {$x}".to_string()),
    };

    let json = serde_json::to_string(&bp)?;
    let deserialized: SourceBreakpoint = serde_json::from_str(&json)?;

    assert_eq!(deserialized.line, 42);
    assert_eq!(deserialized.column, Some(5));
    assert_eq!(deserialized.condition.as_deref(), Some("$x > 10"));
    assert_eq!(deserialized.hit_condition.as_deref(), Some(">= 3"));
    assert!(deserialized.log_message.is_some());
    Ok(())
}

#[test]
fn test_source_breakpoint_minimal() -> Result<(), Box<dyn std::error::Error>> {
    let bp = SourceBreakpoint {
        line: 10,
        column: None,
        condition: None,
        hit_condition: None,
        log_message: None,
    };

    let json = serde_json::to_string(&bp)?;
    // Optional fields should be omitted
    assert!(!json.contains("column"));
    assert!(!json.contains("condition"));
    assert!(!json.contains("hitCondition"));
    assert!(!json.contains("logMessage"));
    Ok(())
}

#[test]
fn test_set_breakpoints_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some("/workspace/script.pl".to_string()),
            name: Some("script.pl".to_string()),
        },
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
                column: Some(5),
                condition: Some("$debug".to_string()),
                hit_condition: None,
                log_message: None,
            },
        ]),
        source_modified: Some(false),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: SetBreakpointsArguments = serde_json::from_str(&json)?;

    assert_eq!(
        deserialized.source.path.as_deref(),
        Some("/workspace/script.pl")
    );
    let bps = deserialized
        .breakpoints
        .as_ref()
        .ok_or("Expected breakpoints")?;
    assert_eq!(bps.len(), 2);
    assert_eq!(bps[0].line, 10);
    assert_eq!(bps[1].line, 20);
    Ok(())
}

#[test]
fn test_capabilities_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let caps = Capabilities {
        supports_configuration_done_request: Some(true),
        supports_evaluate_for_hovers: Some(true),
        supports_conditional_breakpoints: Some(true),
        supports_hit_conditional_breakpoints: Some(true),
        supports_log_points: Some(true),
        supports_exception_options: Some(true),
        supports_exception_filter_options: Some(true),
        supports_terminate_request: Some(true),
        supports_inline_values: Some(false),
        supports_function_breakpoints: Some(true),
        supports_set_variable: Some(true),
        supports_value_formatting_options: Some(false),
        support_terminate_debuggee: Some(true),
        supports_step_back: Some(false),
        supports_data_breakpoints: Some(false),
        exception_breakpoint_filters: Some(vec![ExceptionBreakpointFilter {
            filter: "die".to_string(),
            label: "Break on die/croak".to_string(),
            default: Some(false),
        }]),
    };

    let json = serde_json::to_string(&caps)?;
    let deserialized: Capabilities = serde_json::from_str(&json)?;

    assert_eq!(deserialized.supports_configuration_done_request, Some(true));
    assert_eq!(deserialized.supports_step_back, Some(false));
    let filters = deserialized
        .exception_breakpoint_filters
        .ok_or("Expected filters")?;
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].filter, "die");
    Ok(())
}

#[test]
fn test_stack_trace_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = StackTraceArguments {
        thread_id: 1,
        start_frame: Some(0),
        levels: Some(20),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: StackTraceArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.thread_id, 1);
    assert_eq!(deserialized.start_frame, Some(0));
    assert_eq!(deserialized.levels, Some(20));
    Ok(())
}

#[test]
fn test_stack_trace_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = StackTraceResponseBody {
        stack_frames: vec![
            ProtocolStackFrame {
                id: 0,
                name: "main::run".to_string(),
                source: Some(Source {
                    path: Some("/workspace/script.pl".to_string()),
                    name: Some("script.pl".to_string()),
                }),
                line: 42,
                column: 1,
                end_line: None,
                end_column: None,
            },
            ProtocolStackFrame {
                id: 1,
                name: "My::Module::process".to_string(),
                source: Some(Source {
                    path: Some("/workspace/lib/My/Module.pm".to_string()),
                    name: Some("Module.pm".to_string()),
                }),
                line: 100,
                column: 1,
                end_line: Some(120),
                end_column: Some(1),
            },
        ],
        total_frames: Some(2),
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: StackTraceResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.stack_frames.len(), 2);
    assert_eq!(deserialized.stack_frames[0].name, "main::run");
    assert_eq!(deserialized.stack_frames[1].line, 100);
    assert_eq!(deserialized.total_frames, Some(2));
    Ok(())
}

#[test]
fn test_variables_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = VariablesResponseBody {
        variables: vec![
            ProtocolVariable {
                name: "$x".to_string(),
                value: "42".to_string(),
                type_: Some("SCALAR".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            },
            ProtocolVariable {
                name: "@arr".to_string(),
                value: "[1, 2, 3]".to_string(),
                type_: Some("ARRAY".to_string()),
                variables_reference: 100,
                named_variables: None,
                indexed_variables: Some(3),
            },
        ],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: VariablesResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.variables.len(), 2);
    assert_eq!(deserialized.variables[0].name, "$x");
    assert_eq!(deserialized.variables[0].variables_reference, 0);
    assert_eq!(deserialized.variables[1].indexed_variables, Some(3));
    Ok(())
}

#[test]
fn test_evaluate_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = EvaluateArguments {
        expression: "$hash{key}".to_string(),
        frame_id: Some(0),
        context: Some("hover".to_string()),
        allow_side_effects: Some(false),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: EvaluateArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.expression, "$hash{key}");
    assert_eq!(deserialized.frame_id, Some(0));
    assert_eq!(deserialized.context.as_deref(), Some("hover"));
    assert_eq!(deserialized.allow_side_effects, Some(false));
    Ok(())
}

#[test]
fn test_evaluate_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = EvaluateResponseBody {
        result: "\"hello world\"".to_string(),
        type_: Some("SCALAR".to_string()),
        variables_reference: 0,
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: EvaluateResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.result, "\"hello world\"");
    assert_eq!(deserialized.type_, Some("SCALAR".to_string()));
    Ok(())
}

#[test]
fn test_scopes_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
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
    let deserialized: ScopesResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.scopes.len(), 2);
    assert_eq!(deserialized.scopes[0].name, "Locals");
    assert!(!deserialized.scopes[0].expensive);
    assert!(deserialized.scopes[1].expensive);
    Ok(())
}

#[test]
fn test_continue_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ContinueResponseBody {
        all_threads_continued: true,
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: ContinueResponseBody = serde_json::from_str(&json)?;

    assert!(deserialized.all_threads_continued);
    Ok(())
}

#[test]
fn test_threads_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ThreadsResponseBody {
        threads: vec![Thread {
            id: 1,
            name: "main".to_string(),
        }],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: ThreadsResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.threads.len(), 1);
    assert_eq!(deserialized.threads[0].id, 1);
    assert_eq!(deserialized.threads[0].name, "main");
    Ok(())
}

// ============================================================================
// Control flow argument types (AC9)
// ============================================================================

#[test]
fn test_control_flow_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let continue_args = ContinueArguments { thread_id: 1 };
    let next_args = NextArguments { thread_id: 1 };
    let step_in_args = StepInArguments { thread_id: 1 };
    let step_out_args = StepOutArguments { thread_id: 1 };
    let pause_args = PauseArguments { thread_id: 1 };

    // Verify each round-trips correctly
    let json = serde_json::to_string(&continue_args)?;
    let c: ContinueArguments = serde_json::from_str(&json)?;
    assert_eq!(c.thread_id, 1);

    let json = serde_json::to_string(&next_args)?;
    let n: NextArguments = serde_json::from_str(&json)?;
    assert_eq!(n.thread_id, 1);

    let json = serde_json::to_string(&step_in_args)?;
    let si: StepInArguments = serde_json::from_str(&json)?;
    assert_eq!(si.thread_id, 1);

    let json = serde_json::to_string(&step_out_args)?;
    let so: StepOutArguments = serde_json::from_str(&json)?;
    assert_eq!(so.thread_id, 1);

    let json = serde_json::to_string(&pause_args)?;
    let p: PauseArguments = serde_json::from_str(&json)?;
    assert_eq!(p.thread_id, 1);

    Ok(())
}

// ============================================================================
// Session lifecycle argument types (AC5)
// ============================================================================

#[test]
fn test_disconnect_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = DisconnectArguments {
        restart: Some(false),
        terminate_debuggee: Some(true),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: DisconnectArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.restart, Some(false));
    assert_eq!(deserialized.terminate_debuggee, Some(true));
    Ok(())
}

#[test]
fn test_terminate_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = TerminateArguments {
        restart: Some(true),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: TerminateArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.restart, Some(true));
    Ok(())
}

// ============================================================================
// Extended breakpoint types (AC7)
// ============================================================================

#[test]
fn test_function_breakpoint_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetFunctionBreakpointsArguments {
        breakpoints: vec![
            FunctionBreakpoint {
                name: "main::process".to_string(),
                condition: Some("$debug_mode".to_string()),
                hit_condition: None,
            },
            FunctionBreakpoint {
                name: "My::Module::handler".to_string(),
                condition: None,
                hit_condition: Some(">= 5".to_string()),
            },
        ],
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: SetFunctionBreakpointsArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.breakpoints.len(), 2);
    assert_eq!(deserialized.breakpoints[0].name, "main::process");
    assert!(deserialized.breakpoints[0].condition.is_some());
    assert!(deserialized.breakpoints[1].hit_condition.is_some());
    Ok(())
}

#[test]
fn test_exception_breakpoints_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetExceptionBreakpointsArguments {
        filters: vec!["die".to_string(), "warn".to_string()],
        filter_options: Some(vec![ExceptionFilterOption {
            filter_id: "die".to_string(),
            condition: Some("$_ =~ /fatal/".to_string()),
        }]),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: SetExceptionBreakpointsArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.filters.len(), 2);
    let opts = deserialized
        .filter_options
        .ok_or("Expected filter_options")?;
    assert_eq!(opts.len(), 1);
    assert_eq!(opts[0].filter_id, "die");
    Ok(())
}

// ============================================================================
// Additional protocol types: Modules, Completions, Goto, ExceptionInfo
// ============================================================================

#[test]
fn test_modules_response_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ModulesResponseBody {
        modules: vec![
            Module {
                id: "Foo::Bar".to_string(),
                name: "Foo::Bar".to_string(),
                path: Some("/workspace/lib/Foo/Bar.pm".to_string()),
            },
            Module {
                id: "strict".to_string(),
                name: "strict".to_string(),
                path: None,
            },
        ],
        total_modules: Some(2),
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: ModulesResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.modules.len(), 2);
    assert_eq!(deserialized.modules[0].name, "Foo::Bar");
    assert!(deserialized.modules[1].path.is_none());
    Ok(())
}

#[test]
fn test_completions_response_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = CompletionsResponseBody {
        targets: vec![
            CompletionItem {
                label: "$variable".to_string(),
                type_: Some("variable".to_string()),
                text: None,
                sort_text: None,
                detail: Some("SCALAR".to_string()),
                start: Some(0),
                length: Some(1),
            },
            CompletionItem {
                label: "print".to_string(),
                type_: Some("function".to_string()),
                text: None,
                sort_text: None,
                detail: None,
                start: None,
                length: None,
            },
        ],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: CompletionsResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.targets.len(), 2);
    assert_eq!(deserialized.targets[0].label, "$variable");
    Ok(())
}

#[test]
fn test_goto_targets_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = GotoTargetsResponseBody {
        targets: vec![GotoTarget {
            id: 1,
            label: "line 42".to_string(),
            line: 42,
            column: Some(1),
            end_line: None,
            end_column: None,
        }],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: GotoTargetsResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.targets.len(), 1);
    assert_eq!(deserialized.targets[0].line, 42);
    Ok(())
}

#[test]
fn test_exception_info_response_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = ExceptionInfoResponseBody {
        exception_id: "die".to_string(),
        description: Some("Something went wrong".to_string()),
        break_mode: "always".to_string(),
        details: Some(ExceptionDetails {
            message: Some("Something went wrong at script.pl line 42".to_string()),
            type_name: Some("die".to_string()),
            stack_trace: Some("  at main::run (script.pl:42)".to_string()),
        }),
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: ExceptionInfoResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.exception_id, "die");
    assert_eq!(deserialized.break_mode, "always");
    let details = deserialized.details.ok_or("Expected details")?;
    assert!(details.message.is_some());
    Ok(())
}

// ============================================================================
// Data breakpoint types
// ============================================================================

#[test]
fn test_data_breakpoint_info_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = DataBreakpointInfoResponseBody {
        data_id: Some("$x".to_string()),
        description: "scalar variable $x".to_string(),
        access_types: Some(vec!["write".to_string(), "readWrite".to_string()]),
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: DataBreakpointInfoResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.data_id, Some("$x".to_string()));
    let types = deserialized.access_types.ok_or("Expected access_types")?;
    assert_eq!(types.len(), 2);
    Ok(())
}

#[test]
fn test_set_data_breakpoints_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetDataBreakpointsArguments {
        breakpoints: vec![DataBreakpoint {
            data_id: "$x".to_string(),
            access_type: Some("write".to_string()),
            condition: Some("$x > 100".to_string()),
            hit_condition: None,
        }],
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: SetDataBreakpointsArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.breakpoints.len(), 1);
    assert_eq!(deserialized.breakpoints[0].data_id, "$x");
    Ok(())
}

// ============================================================================
// DapMessage serialization/deserialization
// ============================================================================

#[test]
fn test_dap_message_request_serde() -> Result<(), Box<dyn std::error::Error>> {
    let msg = DapMessage::Request {
        seq: 1,
        command: "initialize".to_string(),
        arguments: Some(json!({"adapterId": "perl-rs"})),
    };

    let json = serde_json::to_string(&msg)?;
    assert!(json.contains("\"type\":\"request\""));
    assert!(json.contains("\"command\":\"initialize\""));

    let deserialized: DapMessage = serde_json::from_str(&json)?;
    match deserialized {
        DapMessage::Request {
            seq,
            command,
            arguments,
        } => {
            assert_eq!(seq, 1);
            assert_eq!(command, "initialize");
            assert!(arguments.is_some());
        }
        _ => return Err("Expected DapMessage::Request".into()),
    }
    Ok(())
}

#[test]
fn test_dap_message_response_serde() -> Result<(), Box<dyn std::error::Error>> {
    let msg = DapMessage::Response {
        seq: 1,
        request_seq: 1,
        success: true,
        command: "initialize".to_string(),
        body: Some(json!({"supportsConfigurationDoneRequest": true})),
        message: None,
    };

    let json = serde_json::to_string(&msg)?;
    assert!(json.contains("\"type\":\"response\""));
    assert!(json.contains("\"success\":true"));

    let deserialized: DapMessage = serde_json::from_str(&json)?;
    match deserialized {
        DapMessage::Response {
            success, command, ..
        } => {
            assert!(success);
            assert_eq!(command, "initialize");
        }
        _ => return Err("Expected DapMessage::Response".into()),
    }
    Ok(())
}

#[test]
fn test_dap_message_event_serde() -> Result<(), Box<dyn std::error::Error>> {
    let msg = DapMessage::Event {
        seq: 1,
        event: "stopped".to_string(),
        body: Some(json!({"reason": "breakpoint", "threadId": 1})),
    };

    let json = serde_json::to_string(&msg)?;
    assert!(json.contains("\"type\":\"event\""));
    assert!(json.contains("\"event\":\"stopped\""));

    let deserialized: DapMessage = serde_json::from_str(&json)?;
    match deserialized {
        DapMessage::Event { event, body, .. } => {
            assert_eq!(event, "stopped");
            assert!(body.is_some());
        }
        _ => return Err("Expected DapMessage::Event".into()),
    }
    Ok(())
}

#[test]
fn test_dap_message_response_error_serde() -> Result<(), Box<dyn std::error::Error>> {
    let msg = DapMessage::Response {
        seq: 2,
        request_seq: 1,
        success: false,
        command: "evaluate".to_string(),
        body: None,
        message: Some("evaluation failed".to_string()),
    };

    let json = serde_json::to_string(&msg)?;
    let deserialized: DapMessage = serde_json::from_str(&json)?;
    match deserialized {
        DapMessage::Response {
            success, message, ..
        } => {
            assert!(!success);
            assert_eq!(message.as_deref(), Some("evaluation failed"));
        }
        _ => return Err("Expected DapMessage::Response".into()),
    }
    Ok(())
}

// ============================================================================
// DapServer / DapConfig / DapMode tests
// ============================================================================

#[test]
fn test_dap_mode_default_is_native() {
    let mode = DapMode::default();
    assert_eq!(mode, DapMode::Native);
}

#[test]
fn test_dap_mode_equality() {
    assert_eq!(DapMode::Native, DapMode::Native);
    assert_eq!(DapMode::Bridge, DapMode::Bridge);
    assert_ne!(DapMode::Native, DapMode::Bridge);
}

#[test]
fn test_dap_mode_clone_and_debug() {
    let mode = DapMode::Bridge;
    let cloned = mode.clone();
    assert_eq!(cloned, DapMode::Bridge);
    let debug_str = format!("{:?}", mode);
    assert!(debug_str.contains("Bridge"));
}

#[test]
fn test_dap_server_creation_native() -> Result<(), Box<dyn std::error::Error>> {
    let config = DapConfig {
        log_level: "info".to_string(),
        mode: DapMode::Native,
        workspace_root: Some(PathBuf::from("/workspace")),
    };
    let server = DapServer::new(config)?;
    assert_eq!(server.config.mode, DapMode::Native);
    assert_eq!(server.config.log_level, "info");
    Ok(())
}

#[test]
fn test_dap_server_creation_bridge() -> Result<(), Box<dyn std::error::Error>> {
    let config = DapConfig {
        log_level: "debug".to_string(),
        mode: DapMode::Bridge,
        workspace_root: None,
    };
    let server = DapServer::new(config)?;
    assert_eq!(server.config.mode, DapMode::Bridge);
    assert!(server.config.workspace_root.is_none());
    Ok(())
}

#[test]
fn test_dap_server_socket_rejects_bridge_mode() -> Result<(), Box<dyn std::error::Error>> {
    let config = DapConfig {
        log_level: "info".to_string(),
        mode: DapMode::Bridge,
        workspace_root: None,
    };
    let mut server = DapServer::new(config)?;
    let result = server.run_socket(0);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not supported in bridge mode"),
        "Expected bridge mode error, got: {}",
        err
    );
    Ok(())
}

// ============================================================================
// BreakpointStore edge cases
// ============================================================================

fn create_test_perl_file() -> (NamedTempFile, String) {
    let mut file = perl_tdd_support::must(NamedTempFile::with_suffix(".pl"));
    let perl_code = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
my $y = 2;
my $z = $x + $y;

if ($x > 0) {
    print "positive\n";
}

my @arr = (1, 2, 3);
while (my $item = shift @arr) {
    my $doubled = $item * 2;
    print "$doubled\n";
}

sub process {
    my ($value) = @_;
    my $result = $value * 2;
    return $result;
}

print "done\n";
my $final = process($x);
print "result: $final\n";
"#;
    let _ = file.write_all(perl_code.as_bytes());
    let _ = file.flush();
    let path = file.path().to_string_lossy().to_string();
    (file, path)
}

#[test]
fn test_breakpoint_store_is_empty() {
    let store = BreakpointStore::new();
    assert!(store.is_empty());

    let (_file, source_path) = create_test_perl_file();
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some(source_path.clone()),
            name: None,
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 5,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }]),
        source_modified: None,
    };
    store.set_breakpoints(&args);
    assert!(!store.is_empty());

    store.clear_all();
    assert!(store.is_empty());
}

#[test]
fn test_breakpoint_record_to_protocol() {
    let record = BreakpointRecord {
        id: 42,
        line: 10,
        column: Some(5),
        condition: Some("$x > 0".to_string()),
        hit_condition: None,
        log_message: None,
        hit_count: 3,
        verified: true,
        message: Some("adjusted to executable line".to_string()),
    };

    let protocol_bp = record.to_protocol();

    assert_eq!(protocol_bp.id, 42);
    assert_eq!(protocol_bp.line, 10);
    assert_eq!(protocol_bp.column, Some(5));
    assert!(protocol_bp.verified);
    assert_eq!(
        protocol_bp.message.as_deref(),
        Some("adjusted to executable line")
    );
}

#[test]
fn test_breakpoint_record_to_protocol_unverified() {
    let record = BreakpointRecord {
        id: 1,
        line: 3,
        column: None,
        condition: None,
        hit_condition: None,
        log_message: None,
        hit_count: 0,
        verified: false,
        message: Some("comment line".to_string()),
    };

    let protocol_bp = record.to_protocol();

    assert_eq!(protocol_bp.id, 1);
    assert!(!protocol_bp.verified);
    assert!(protocol_bp.column.is_none());
}

#[test]
fn test_breakpoint_hit_outcome_default() {
    let outcome = perl_dap::breakpoints::BreakpointHitOutcome::default();
    assert!(!outcome.matched);
    assert!(!outcome.should_stop);
    assert!(outcome.log_messages.is_empty());
}

#[test]
fn test_register_breakpoint_hit_no_match() {
    let store = BreakpointStore::new();
    // No breakpoints registered at all
    let outcome = store.register_breakpoint_hit("/some/file.pl", 10);
    assert!(!outcome.matched);
    assert!(!outcome.should_stop);
    assert!(outcome.log_messages.is_empty());
}

#[test]
fn test_register_breakpoint_hit_unverified_breakpoint_not_matched() {
    let store = BreakpointStore::new();
    // Set a breakpoint on a file that doesn't exist (will be unverified)
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some("/nonexistent/file.pl".to_string()),
            name: None,
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 10,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }]),
        source_modified: None,
    };
    let bps = store.set_breakpoints(&args);
    assert!(!bps.is_empty());
    assert!(!bps[0].verified);

    // Hitting the line should not match because breakpoint is unverified
    let outcome = store.register_breakpoint_hit("/nonexistent/file.pl", 10);
    assert!(!outcome.matched);
}

#[test]
fn test_adjust_breakpoints_for_edit_negative_delta_clamps_to_one() {
    let store = BreakpointStore::new();

    // Use set_breakpoints with a real temp file
    let (_file, source_path) = create_test_perl_file();
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some(source_path.clone()),
            name: None,
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 5,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }]),
        source_modified: None,
    };
    store.set_breakpoints(&args);

    // Removing 10 lines at line 2 should push bp from line 5 down to clamped value
    store.adjust_breakpoints_for_edit(&source_path, 2, -10);
    let bps = store.get_breakpoints(&source_path);
    assert_eq!(bps.len(), 1);
    // Line 5 - 10 = -5, clamped to 1
    assert_eq!(bps[0].line, 1);
    assert!(!bps[0].verified, "Breakpoint should be invalidated by edit");
    assert!(
        bps[0]
            .message
            .as_ref()
            .is_some_and(|m| m.contains("invalidated")),
        "Should have invalidation message"
    );
}

#[test]
fn test_breakpoint_store_negative_line_rejected() {
    let (_file, source_path) = create_test_perl_file();
    let store = BreakpointStore::new();
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some(source_path),
            name: None,
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: -1,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
        }]),
        source_modified: None,
    };
    let bps = store.set_breakpoints(&args);
    assert_eq!(bps.len(), 1);
    assert!(!bps[0].verified);
    assert!(
        bps[0]
            .message
            .as_ref()
            .is_some_and(|m| m.contains("positive")),
        "Should mention line must be positive"
    );
}

#[test]
fn test_breakpoint_condition_with_newline_rejected() {
    let (_file, source_path) = create_test_perl_file();
    let store = BreakpointStore::new();
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some(source_path),
            name: None,
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 5,
            column: None,
            condition: Some("1\nprint 'injected'".to_string()),
            hit_condition: None,
            log_message: None,
        }]),
        source_modified: None,
    };
    let bps = store.set_breakpoints(&args);
    assert_eq!(bps.len(), 1);
    assert!(!bps[0].verified);
    assert!(
        bps[0]
            .message
            .as_ref()
            .is_some_and(|m| m.contains("newline")),
        "Should mention newlines are not allowed"
    );
}

#[test]
fn test_breakpoint_invalid_hit_condition_rejected() {
    let (_file, source_path) = create_test_perl_file();
    let store = BreakpointStore::new();
    let args = SetBreakpointsArguments {
        source: Source {
            path: Some(source_path),
            name: None,
        },
        breakpoints: Some(vec![SourceBreakpoint {
            line: 5,
            column: None,
            condition: None,
            hit_condition: Some("not_a_number".to_string()),
            log_message: None,
        }]),
        source_modified: None,
    };
    let bps = store.set_breakpoints(&args);
    assert_eq!(bps.len(), 1);
    assert!(!bps[0].verified);
    assert!(
        bps[0]
            .message
            .as_ref()
            .is_some_and(|m| m.contains("Invalid hitCondition")),
        "Should mention invalid hit condition"
    );
}

// ============================================================================
// Inline values edge cases
// ============================================================================

#[test]
fn test_inline_values_empty_source() {
    let values = perl_dap::inline_values::collect_inline_values("", 1, 1);
    assert!(values.is_empty());
}

#[test]
fn test_inline_values_out_of_range() {
    let source = "my $x = 1;\nmy $y = 2;\n";
    // Start line beyond file length
    let values = perl_dap::inline_values::collect_inline_values(source, 100, 200);
    assert!(values.is_empty());
}

#[test]
fn test_inline_values_single_line() {
    let source = "my $total = $x + $y;";
    let values = perl_dap::inline_values::collect_inline_values(source, 1, 1);
    assert!(!values.is_empty());
    assert!(values.iter().any(|v| v.text.contains("$total")));
    assert!(values.iter().any(|v| v.text.contains("$x")));
    assert!(values.iter().any(|v| v.text.contains("$y")));
}

#[test]
fn test_inline_values_line_and_column_are_one_based() {
    let source = "my $x = 1;\n";
    let values = perl_dap::inline_values::collect_inline_values(source, 1, 1);
    assert!(!values.is_empty());
    // line should be 1-based
    assert!(values.iter().all(|v| v.line >= 1));
    // column should be 1-based
    assert!(values.iter().all(|v| v.column >= 1));
}

#[test]
fn test_inline_values_no_variables() {
    let source = "use strict;\nuse warnings;\n# comment\n";
    let values = perl_dap::inline_values::collect_inline_values(source, 1, 3);
    // These lines have no scalar variables to extract
    // (use strict/warnings might match $_ or other patterns depending on regex)
    // Just verify it doesn't panic
    let _ = values;
}

// ============================================================================
// DebugAdapter construction
// ============================================================================

#[test]
fn test_debug_adapter_new() {
    let _adapter = DebugAdapter::new();
    // Just verify it constructs successfully without panic
}

// ============================================================================
// Feature catalog tests
// ============================================================================

#[test]
fn test_feature_catalog_has_feature_known() {
    let result = perl_dap::feature_catalog::has_feature("dap.breakpoints.basic");
    assert!(
        result,
        "dap.breakpoints.basic should be a registered feature"
    );
}

#[test]
fn test_feature_catalog_all_dap_features_registered() {
    let all_ids = [
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
    for id in all_ids {
        assert!(
            perl_dap::feature_catalog::has_feature(id),
            "feature `{id}` should be registered in the DAP catalog"
        );
    }
}

#[test]
fn test_feature_catalog_has_feature_unknown() {
    let result = perl_dap::feature_catalog::has_feature("nonexistent.feature.xyz");
    assert!(!result, "Unknown feature should return false");
}

#[test]
fn test_feature_catalog_advertised_features_not_empty() {
    let features = perl_dap::feature_catalog::advertised_features();
    assert!(
        !features.is_empty(),
        "Should have at least one advertised feature"
    );
}

// ============================================================================
// Launch/Attach configuration edge cases
// ============================================================================

#[test]
fn test_launch_config_resolve_paths_no_cwd() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = LaunchConfiguration {
        program: PathBuf::from("script.pl"),
        args: vec![],
        cwd: None,
        env: HashMap::new(),
        perl_path: None,
        include_paths: vec![],
    };

    config.resolve_paths(&PathBuf::from("/workspace"))?;
    assert_eq!(config.program, PathBuf::from("/workspace/script.pl"));
    assert!(config.cwd.is_none());
    Ok(())
}

#[test]
fn test_attach_config_max_valid_port() -> Result<(), Box<dyn std::error::Error>> {
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 65535,
        timeout_ms: Some(5000),
        stop_on_entry: None,
    };
    config.validate()?;
    Ok(())
}

#[test]
fn test_attach_config_min_valid_port() -> Result<(), Box<dyn std::error::Error>> {
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 1,
        timeout_ms: Some(5000),
        stop_on_entry: None,
    };
    config.validate()?;
    Ok(())
}

#[test]
fn test_attach_config_boundary_timeout() -> Result<(), Box<dyn std::error::Error>> {
    // Exactly 300000 (5 minutes) should be valid
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(300_000),
        stop_on_entry: None,
    };
    config.validate()?;

    // 300001 should fail
    let config = AttachConfiguration {
        host: "localhost".to_string(),
        port: 13603,
        timeout_ms: Some(300_001),
        stop_on_entry: None,
    };
    assert!(config.validate().is_err());
    Ok(())
}

// ============================================================================
// Breakpoint verification fields round-trip
// ============================================================================

#[test]
fn test_breakpoint_protocol_type_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let bp = Breakpoint {
        id: 1,
        verified: true,
        line: 42,
        column: Some(5),
        message: Some("adjusted".to_string()),
    };

    let json = serde_json::to_string(&bp)?;
    let deserialized: Breakpoint = serde_json::from_str(&json)?;

    assert_eq!(deserialized.id, 1);
    assert!(deserialized.verified);
    assert_eq!(deserialized.line, 42);
    assert_eq!(deserialized.column, Some(5));
    assert_eq!(deserialized.message.as_deref(), Some("adjusted"));
    Ok(())
}

#[test]
fn test_breakpoint_locations_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = BreakpointLocationsResponseBody {
        breakpoints: vec![
            BreakpointLocation {
                line: 10,
                column: Some(1),
                end_line: Some(10),
                end_column: Some(20),
            },
            BreakpointLocation {
                line: 15,
                column: None,
                end_line: None,
                end_column: None,
            },
        ],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: BreakpointLocationsResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.breakpoints.len(), 2);
    assert_eq!(deserialized.breakpoints[0].line, 10);
    assert!(deserialized.breakpoints[1].column.is_none());
    Ok(())
}

// ============================================================================
// SetVariable / SetExpression round-trips
// ============================================================================

#[test]
fn test_set_variable_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetVariableArguments {
        variables_reference: 100,
        name: "$x".to_string(),
        value: "42".to_string(),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: SetVariableArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.variables_reference, 100);
    assert_eq!(deserialized.name, "$x");
    assert_eq!(deserialized.value, "42");
    Ok(())
}

#[test]
fn test_set_expression_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = SetExpressionArguments {
        expression: "$hash{key}".to_string(),
        value: "\"new value\"".to_string(),
        frame_id: Some(0),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: SetExpressionArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.expression, "$hash{key}");
    assert_eq!(deserialized.value, "\"new value\"");
    assert_eq!(deserialized.frame_id, Some(0));
    Ok(())
}

// ============================================================================
// Cancel / RestartFrame / TerminateThreads arguments
// ============================================================================

#[test]
fn test_cancel_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = CancelArguments {
        request_id: Some(42),
        progress_id: Some("progress-1".to_string()),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: CancelArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.request_id, Some(42));
    assert_eq!(deserialized.progress_id.as_deref(), Some("progress-1"));
    Ok(())
}

#[test]
fn test_restart_frame_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = RestartFrameArguments { frame_id: 3 };

    let json = serde_json::to_string(&args)?;
    let deserialized: RestartFrameArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.frame_id, 3);
    Ok(())
}

#[test]
fn test_terminate_threads_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = TerminateThreadsArguments {
        thread_ids: Some(vec![1, 2, 3]),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: TerminateThreadsArguments = serde_json::from_str(&json)?;

    let ids = deserialized.thread_ids.ok_or("Expected thread_ids")?;
    assert_eq!(ids, vec![1, 2, 3]);
    Ok(())
}

// ============================================================================
// Source / LoadedSources / Restart types
// ============================================================================

#[test]
fn test_source_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = SourceResponseBody {
        content: "#!/usr/bin/perl\nprint 'hello';\n".to_string(),
        mime_type: Some("text/x-perl".to_string()),
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: SourceResponseBody = serde_json::from_str(&json)?;

    assert!(deserialized.content.contains("perl"));
    assert_eq!(deserialized.mime_type.as_deref(), Some("text/x-perl"));
    Ok(())
}

#[test]
fn test_loaded_sources_response_body_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = LoadedSourcesResponseBody {
        sources: vec![
            Source {
                path: Some("/workspace/script.pl".to_string()),
                name: Some("script.pl".to_string()),
            },
            Source {
                path: Some("/usr/lib/perl5/strict.pm".to_string()),
                name: Some("strict.pm".to_string()),
            },
        ],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: LoadedSourcesResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.sources.len(), 2);
    Ok(())
}

#[test]
fn test_restart_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = RestartArguments {
        arguments: Some(json!({"program": "script.pl"})),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: RestartArguments = serde_json::from_str(&json)?;

    assert!(deserialized.arguments.is_some());
    Ok(())
}

// ============================================================================
// StepInTargets type
// ============================================================================

#[test]
fn test_step_in_targets_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let body = StepInTargetsResponseBody {
        targets: vec![
            StepInTarget {
                id: 1,
                label: "My::Module::process".to_string(),
            },
            StepInTarget {
                id: 2,
                label: "closure at line 42".to_string(),
            },
        ],
    };

    let json = serde_json::to_string(&body)?;
    let deserialized: StepInTargetsResponseBody = serde_json::from_str(&json)?;

    assert_eq!(deserialized.targets.len(), 2);
    assert_eq!(deserialized.targets[0].label, "My::Module::process");
    Ok(())
}

// ============================================================================
// Initialize request arguments
// ============================================================================

#[test]
fn test_initialize_request_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
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
    let deserialized: InitializeRequestArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.adapter_id, "perl-rs");
    assert_eq!(deserialized.client_id.as_deref(), Some("vscode"));
    assert_eq!(deserialized.lines_start_at1, Some(true));
    Ok(())
}

// ============================================================================
// Launch/Attach request arguments (protocol level)
// ============================================================================

#[test]
fn test_launch_request_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = LaunchRequestArguments {
        program: "/workspace/script.pl".to_string(),
        args: Some(vec!["--verbose".to_string()]),
        cwd: Some("/workspace".to_string()),
        env: Some(HashMap::from([("DEBUG".to_string(), "1".to_string())])),
        perl_path: Some("/usr/bin/perl".to_string()),
        stop_on_entry: Some(true),
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: LaunchRequestArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.program, "/workspace/script.pl");
    assert_eq!(deserialized.stop_on_entry, Some(true));
    let env = deserialized.env.ok_or("Expected env")?;
    assert_eq!(env.get("DEBUG").map(String::as_str), Some("1"));
    Ok(())
}

#[test]
fn test_attach_request_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = AttachRequestArguments {
        process_id: Some(1234),
        host: Some("localhost".to_string()),
        port: Some(13603),
        timeout: Some(5000),
        stop_on_entry: None,
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: AttachRequestArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.process_id, Some(1234));
    assert_eq!(deserialized.host.as_deref(), Some("localhost"));
    assert_eq!(deserialized.port, Some(13603));
    Ok(())
}

// ============================================================================
// Inline values argument types
// ============================================================================

#[test]
fn test_inline_values_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = InlineValuesArguments {
        source: Source {
            path: Some("/workspace/script.pl".to_string()),
            name: None,
        },
        start_line: 1,
        end_line: 50,
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: InlineValuesArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.start_line, 1);
    assert_eq!(deserialized.end_line, 50);
    Ok(())
}

// ============================================================================
// GotoArguments round-trip
// ============================================================================

#[test]
fn test_goto_arguments_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let args = GotoArguments {
        thread_id: 1,
        target_id: 42,
    };

    let json = serde_json::to_string(&args)?;
    let deserialized: GotoArguments = serde_json::from_str(&json)?;

    assert_eq!(deserialized.thread_id, 1);
    assert_eq!(deserialized.target_id, 42);
    Ok(())
}
