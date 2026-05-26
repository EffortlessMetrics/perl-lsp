//! Property tests: DAP protocol message JSON round-trip invariants.
//!
//! Invariants tested:
//!
//! 1. **Request round-trip** - any constructed `Request` survives JSON encode/decode
//!    with all fields preserved.
//! 2. **Response round-trip** - `Response` (success and error paths) survives encode/decode.
//! 3. **Event round-trip** - `Event` survives encode/decode.
//! 4. **SourceBreakpoint fields preserved** - all optional fields that are `Some` come
//!    back as `Some` after a round-trip.
//! 5. **Breakpoint (response) fields preserved** - `id`, `verified`, `line` are stable.
//! 6. **Idempotent re-serialization** - serializing the decoded value produces the
//!    same JSON as the first serialization (serialize -> deserialize -> serialize == first).
//! 7. **Optional-field omission** - `None` fields must not appear in the JSON payload
//!    (checked on a sample of types with `skip_serializing_if`).
//! 8. **Capabilities round-trip** - arbitrarily-populated `Capabilities` survive
//!    encode/decode with boolean fields stable.
//! 9. **LaunchRequestArguments round-trip** - all fields preserved, including `env` map.
//! 10. **AttachRequestArguments round-trip** - TCP vs PID modes both survive.
//! 11. **Thread / Scope / ProtocolVariable round-trip** - response body types survive.
//! 12. **Module round-trip** - `Module` struct fields preserved.

use perl_dap::protocol::{
    AttachRequestArguments, Breakpoint, Capabilities, Event, ExceptionBreakpointFilter,
    LaunchRequestArguments, Module, ProtocolVariable, Request, Response, Scope, SourceBreakpoint,
    Thread,
};
use proptest::prelude::*;

// --- Strategy helpers --------------------------------------------------------

/// Strategy for a serde_json::Value used as an optional body/arguments field.
///
/// NOTE: `serde_json::Value::Null` is intentionally excluded here.  When an
/// `Option<serde_json::Value>` field is `Some(Null)`, serde serializes it as
/// `"field":null`.  On deserialization, serde maps JSON `null` back to `None`
/// for an `Option<T>` field, so `Some(Null)` -> `None` is a known lossy path.
/// Using only non-null leaf values keeps round-trip assertions well-defined.
fn non_null_leaf_json_value() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        any::<bool>().prop_map(serde_json::Value::Bool),
        (1i64..1000).prop_map(|n| serde_json::Value::Number(n.into())),
        "[\\PC]{1,32}".prop_map(serde_json::Value::String),
    ]
}

// --- Invariant 1: Request round-trip ----------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_request_json_roundtrip(
        seq in 0i64..10_000,
        command in "[a-zA-Z][a-zA-Z]{0,31}",
        has_args in any::<bool>(),
        arg_value in non_null_leaf_json_value(),
    ) {
        let arguments = if has_args { Some(arg_value) } else { None };
        let req = Request {
            seq,
            msg_type: "request".to_string(),
            command: command.clone(),
            arguments: arguments.clone(),
        };

        let json_str = serde_json::to_string(&req)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Request = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.seq, seq);
        prop_assert_eq!(&back.msg_type, "request");
        prop_assert_eq!(&back.command, &command);
        prop_assert_eq!(back.arguments.is_some(), arguments.is_some());

        // Invariant 6: idempotent re-serialization
        let json_str2 = serde_json::to_string(&back)
            .map_err(|e| TestCaseError::fail(format!("re-serialize failed: {e}")))?;
        prop_assert_eq!(&json_str, &json_str2, "re-serialization must be idempotent");
    }
}

// --- Invariant 2: Response round-trip ---------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_response_json_roundtrip(
        seq in 0i64..10_000,
        request_seq in 0i64..10_000,
        success in any::<bool>(),
        command in "[a-zA-Z][a-zA-Z]{0,31}",
        has_message in any::<bool>(),
        message_val in "[\\PC]{0,64}",
        has_body in any::<bool>(),
        body_value in non_null_leaf_json_value(),
    ) {
        let message = if !success && has_message { Some(message_val.clone()) } else { None };
        let body = if has_body { Some(body_value) } else { None };

        let resp = Response {
            seq,
            msg_type: "response".to_string(),
            request_seq,
            success,
            command: command.clone(),
            message: message.clone(),
            body: body.clone(),
        };

        let json_str = serde_json::to_string(&resp)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Response = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.seq, seq);
        prop_assert_eq!(back.request_seq, request_seq);
        prop_assert_eq!(back.success, success);
        prop_assert_eq!(&back.command, &command);
        prop_assert_eq!(&back.message, &message);
        prop_assert_eq!(back.body.is_some(), body.is_some());

        // Invariant 6: idempotent re-serialization
        let json_str2 = serde_json::to_string(&back)
            .map_err(|e| TestCaseError::fail(format!("re-serialize failed: {e}")))?;
        prop_assert_eq!(&json_str, &json_str2, "re-serialization must be idempotent");
    }
}

// --- Invariant 3: Event round-trip ------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_event_json_roundtrip(
        seq in 0i64..10_000,
        event_name in "[a-zA-Z][a-zA-Z]{0,31}",
        has_body in any::<bool>(),
        body_value in non_null_leaf_json_value(),
    ) {
        let body = if has_body { Some(body_value) } else { None };

        let evt = Event {
            seq,
            msg_type: "event".to_string(),
            event: event_name.clone(),
            body: body.clone(),
        };

        let json_str = serde_json::to_string(&evt)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Event = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.seq, seq);
        prop_assert_eq!(&back.msg_type, "event");
        prop_assert_eq!(&back.event, &event_name);
        prop_assert_eq!(back.body.is_some(), body.is_some());

        // Invariant 6: idempotent re-serialization
        let json_str2 = serde_json::to_string(&back)
            .map_err(|e| TestCaseError::fail(format!("re-serialize failed: {e}")))?;
        prop_assert_eq!(&json_str, &json_str2, "re-serialization must be idempotent");
    }
}

// --- Invariant 4: SourceBreakpoint fields preserved -------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_source_breakpoint_fields_preserved(
        line in 1i64..10_000,
        column in proptest::option::of(0i64..500),
        condition in proptest::option::of("[\\PC]{0,64}"),
        hit_condition in proptest::option::of("[\\PC]{0,32}"),
        log_message in proptest::option::of("[\\PC]{0,64}"),
    ) {
        let bp = SourceBreakpoint {
            line,
            column,
            condition: condition.clone(),
            hit_condition: hit_condition.clone(),
            log_message: log_message.clone(),
        };

        let json_str = serde_json::to_string(&bp)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: SourceBreakpoint = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.line, line);
        prop_assert_eq!(back.column, column);
        prop_assert_eq!(&back.condition, &condition);
        prop_assert_eq!(&back.hit_condition, &hit_condition);
        prop_assert_eq!(&back.log_message, &log_message);

        // Invariant 7: None fields must not appear in JSON
        if column.is_none() {
            prop_assert!(
                !json_str.contains("\"column\""),
                "None column must be omitted, got: {json_str}"
            );
        }
        if condition.is_none() {
            prop_assert!(
                !json_str.contains("\"condition\""),
                "None condition must be omitted, got: {json_str}"
            );
        }
    }
}

// --- Invariant 5: Breakpoint (response) fields preserved --------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_breakpoint_response_fields_preserved(
        id in 1i64..100_000,
        verified in any::<bool>(),
        line in 1i64..10_000,
        column in proptest::option::of(0i64..500),
        message in proptest::option::of("[\\PC]{0,64}"),
    ) {
        let bp = Breakpoint {
            id,
            verified,
            line,
            column,
            message: message.clone(),
        };

        let json_str = serde_json::to_string(&bp)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Breakpoint = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.id, id);
        prop_assert_eq!(back.verified, verified);
        prop_assert_eq!(back.line, line);
        prop_assert_eq!(back.column, column);
        prop_assert_eq!(&back.message, &message);

        // Invariant 7: None fields must not appear in JSON
        if column.is_none() {
            prop_assert!(
                !json_str.contains("\"column\""),
                "None column must be omitted, got: {json_str}"
            );
        }
        if message.is_none() {
            prop_assert!(
                !json_str.contains("\"message\""),
                "None message must be omitted, got: {json_str}"
            );
        }
    }
}

// --- Invariant 8: Capabilities round-trip -----------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_capabilities_boolean_fields_stable(
        config_done in proptest::option::of(any::<bool>()),
        eval_hovers in proptest::option::of(any::<bool>()),
        conditional_bp in proptest::option::of(any::<bool>()),
        hit_conditional_bp in proptest::option::of(any::<bool>()),
        log_points in proptest::option::of(any::<bool>()),
        exception_options in proptest::option::of(any::<bool>()),
        terminate_req in proptest::option::of(any::<bool>()),
        inline_values in proptest::option::of(any::<bool>()),
        set_variable in proptest::option::of(any::<bool>()),
        step_back in proptest::option::of(any::<bool>()),
        data_breakpoints in proptest::option::of(any::<bool>()),
    ) {
        let caps = Capabilities {
            supports_configuration_done_request: config_done,
            supports_evaluate_for_hovers: eval_hovers,
            supports_conditional_breakpoints: conditional_bp,
            supports_hit_conditional_breakpoints: hit_conditional_bp,
            supports_log_points: log_points,
            supports_exception_options: exception_options,
            supports_exception_filter_options: None,
            supports_terminate_request: terminate_req,
            supports_inline_values: inline_values,
            supports_function_breakpoints: None,
            supports_set_variable: set_variable,
            supports_value_formatting_options: None,
            support_terminate_debuggee: None,
            supports_step_back: step_back,
            supports_data_breakpoints: data_breakpoints,
            exception_breakpoint_filters: None,
        };

        let json_str = serde_json::to_string(&caps)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Capabilities = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.supports_configuration_done_request, config_done);
        prop_assert_eq!(back.supports_evaluate_for_hovers, eval_hovers);
        prop_assert_eq!(back.supports_conditional_breakpoints, conditional_bp);
        prop_assert_eq!(back.supports_log_points, log_points);
        prop_assert_eq!(back.supports_set_variable, set_variable);
        prop_assert_eq!(back.supports_step_back, step_back);
        prop_assert_eq!(back.supports_data_breakpoints, data_breakpoints);

        // Invariant 7: None fields must not appear in JSON
        if config_done.is_none() {
            prop_assert!(
                !json_str.contains("\"supportsConfigurationDoneRequest\""),
                "None field must be omitted: {json_str}"
            );
        }
    }
}

// --- Invariant 9: LaunchRequestArguments round-trip -------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_launch_request_args_roundtrip(
        program in "[/a-zA-Z0-9._]{1,64}",
        args_vec in proptest::collection::vec("[\\PC]{0,32}", 0..8usize),
        cwd in proptest::option::of("[/a-zA-Z0-9._]{1,64}"),
        perl_path in proptest::option::of("[/a-zA-Z0-9._]{1,64}"),
        stop_on_entry in proptest::option::of(any::<bool>()),
        env_pairs in proptest::collection::vec(("[A-Z_]{1,16}", "[\\PC]{0,32}"), 0..8usize),
    ) {
        let has_args = !args_vec.is_empty();
        let env: std::collections::HashMap<String, String> = env_pairs.into_iter().collect();
        let has_env = !env.is_empty();

        let launch_args = LaunchRequestArguments {
            program: program.clone(),
            args: if has_args { Some(args_vec.clone()) } else { None },
            cwd: cwd.clone(),
            env: if has_env { Some(env.clone()) } else { None },
            perl_path: perl_path.clone(),
            stop_on_entry,
        };

        let json_str = serde_json::to_string(&launch_args)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: LaunchRequestArguments = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(&back.program, &program);
        prop_assert_eq!(&back.cwd, &cwd);
        prop_assert_eq!(&back.perl_path, &perl_path);
        prop_assert_eq!(back.stop_on_entry, stop_on_entry);

        if has_args {
            prop_assert_eq!(back.args.as_deref(), Some(args_vec.as_slice()));
        } else {
            prop_assert!(back.args.is_none());
        }

        if has_env {
            prop_assert_eq!(&back.env, &Some(env));
        } else {
            prop_assert!(back.env.is_none());
        }
    }
}

// --- Invariant 10: AttachRequestArguments round-trip ------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_attach_request_args_tcp_mode_roundtrip(
        host in "[a-zA-Z0-9.-]{1,32}",
        port in 1024u16..65535,
        timeout in proptest::option::of(0u32..30_000),
        stop_on_entry in proptest::option::of(any::<bool>()),
    ) {
        let attach_args = AttachRequestArguments {
            process_id: None,
            host: Some(host.clone()),
            port: Some(port),
            timeout,
            stop_on_entry,
        };

        let json_str = serde_json::to_string(&attach_args)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: AttachRequestArguments = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.host.as_deref(), Some(host.as_str()));
        prop_assert_eq!(back.port, Some(port));
        prop_assert_eq!(back.timeout, timeout);
        prop_assert_eq!(back.stop_on_entry, stop_on_entry);
        prop_assert!(back.process_id.is_none());
    }

    #[test]
    fn prop_attach_request_args_pid_mode_roundtrip(
        pid in 1u32..65535,
        stop_on_entry in proptest::option::of(any::<bool>()),
    ) {
        let attach_args = AttachRequestArguments {
            process_id: Some(pid),
            host: None,
            port: None,
            timeout: None,
            stop_on_entry,
        };

        let json_str = serde_json::to_string(&attach_args)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: AttachRequestArguments = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.process_id, Some(pid));
        prop_assert!(back.host.is_none());
        prop_assert!(back.port.is_none());
        prop_assert!(back.timeout.is_none());
        prop_assert_eq!(back.stop_on_entry, stop_on_entry);

        // Invariant 7: None optional fields must be absent from JSON
        prop_assert!(
            !json_str.contains("\"host\""),
            "None host must be omitted: {json_str}"
        );
    }
}

// --- Invariant 11: Thread / Scope / ProtocolVariable round-trip -------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_thread_roundtrip(
        id in 1i64..1000,
        name in "[\\PC]{1,32}",
    ) {
        let thread = Thread { id, name: name.clone() };

        let json_str = serde_json::to_string(&thread)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Thread = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(back.id, id);
        prop_assert_eq!(&back.name, &name);
    }

    #[test]
    fn prop_scope_roundtrip(
        name in "[\\PC]{1,32}",
        variables_reference in 1i64..100_000,
        expensive in any::<bool>(),
        presentation_hint in proptest::option::of("arguments|locals|registers"),
    ) {
        let scope = Scope {
            name: name.clone(),
            presentation_hint: presentation_hint.clone(),
            variables_reference,
            expensive,
        };

        let json_str = serde_json::to_string(&scope)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Scope = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(&back.name, &name);
        prop_assert_eq!(back.variables_reference, variables_reference);
        prop_assert_eq!(back.expensive, expensive);
        prop_assert_eq!(&back.presentation_hint, &presentation_hint);
    }

    #[test]
    fn prop_protocol_variable_roundtrip(
        name in "[\\PC]{1,32}",
        value in "[\\PC]{0,64}",
        type_hint in proptest::option::of("[A-Z]{2,10}"),
        variables_reference in 0i64..1000,
        named_variables in proptest::option::of(0i64..100),
        indexed_variables in proptest::option::of(0i64..1000),
    ) {
        let var = ProtocolVariable {
            name: name.clone(),
            value: value.clone(),
            type_: type_hint.clone(),
            variables_reference,
            named_variables,
            indexed_variables,
        };

        let json_str = serde_json::to_string(&var)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: ProtocolVariable = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(&back.name, &name);
        prop_assert_eq!(&back.value, &value);
        prop_assert_eq!(&back.type_, &type_hint);
        prop_assert_eq!(back.variables_reference, variables_reference);
        prop_assert_eq!(back.named_variables, named_variables);
        prop_assert_eq!(back.indexed_variables, indexed_variables);

        // Invariant 7: type_ uses "type" key in JSON (not "type_")
        if var.type_.is_some() {
            prop_assert!(
                json_str.contains("\"type\""),
                "type_ field must serialize as 'type': {json_str}"
            );
            prop_assert!(
                !json_str.contains("\"type_\""),
                "type_ key must not appear in JSON: {json_str}"
            );
        }
    }
}

// --- Invariant 12: Module round-trip ----------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_module_roundtrip(
        id in "[a-zA-Z0-9:_]{1,32}",
        name in "[a-zA-Z0-9:_]{1,32}",
        path in proptest::option::of("[/a-zA-Z0-9._]{1,64}"),
    ) {
        let module = Module {
            id: id.clone(),
            name: name.clone(),
            path: path.clone(),
        };

        let json_str = serde_json::to_string(&module)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: Module = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(&back.id, &id);
        prop_assert_eq!(&back.name, &name);
        prop_assert_eq!(&back.path, &path);

        // Invariant 7: None path must be absent from JSON
        if path.is_none() {
            prop_assert!(
                !json_str.contains("\"path\""),
                "None path must be omitted: {json_str}"
            );
        }
    }
}

// --- Invariant (extra): ExceptionBreakpointFilter round-trip ----------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_exception_breakpoint_filter_roundtrip(
        filter_id in "[a-z_]{1,16}",
        label in "[\\PC]{1,48}",
        default in proptest::option::of(any::<bool>()),
    ) {
        let filter = ExceptionBreakpointFilter {
            filter: filter_id.clone(),
            label: label.clone(),
            default,
        };

        let json_str = serde_json::to_string(&filter)
            .map_err(|e| TestCaseError::fail(format!("serialize failed: {e}")))?;
        let back: ExceptionBreakpointFilter = serde_json::from_str(&json_str)
            .map_err(|e| TestCaseError::fail(format!("deserialize failed: {e}")))?;

        prop_assert_eq!(&back.filter, &filter_id);
        prop_assert_eq!(&back.label, &label);
        prop_assert_eq!(back.default, default);
    }
}
