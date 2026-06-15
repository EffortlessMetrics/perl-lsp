//! REPL and expression evaluation: evaluate, set expression, completions.

use super::*;
use std::sync::LazyLock;

static SAFE_EVALUATOR: LazyLock<SafeEvaluator> = LazyLock::new(SafeEvaluator::new);

impl DebugAdapter {
    /// Handle evaluate request with policy validation and timeout enforcement.
    ///
    /// AC10.1: Evaluates expressions in stack frame context
    /// AC10.2: Policy validation for a non-mutating subset by default
    /// AC10.3: Timeout enforcement (5s default, 30s hard limit)
    pub(super) fn handle_evaluate(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: EvaluateArguments = match arguments.and_then(|v| serde_json::from_value(v).ok()) {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("Missing arguments".to_string()),
                };
            }
        };

        {
            let expression = &args.expression;

            if expression.is_empty() {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("Empty expression".to_string()),
                };
            }

            // Security: Reject expressions with newlines to prevent command injection
            if expression.contains('\n') || expression.contains('\r') {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("Expression cannot contain newlines".to_string()),
                };
            }

            // AC10.2: Policy validation for a non-mutating subset by default.
            // This is admission control, not a real sandbox.
            let allow_side_effects = args.allow_side_effects.unwrap_or(false);

            // Validate expression safety if side effects are not allowed
            if !allow_side_effects {
                if let Some(error) = validate_safe_expression(expression) {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "evaluate".to_string(),
                        body: None,
                        message: Some(error),
                    };
                }

                // Re-run through microcrate validator to keep evaluation policy aligned
                // with shared DAP security logic.
                if let Err(error) = SAFE_EVALUATOR.validate(expression) {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "evaluate".to_string(),
                        body: None,
                        message: Some(error.to_string()),
                    };
                }
            }
        }

        // Validate frameId when provided: the frame must exist in the current session and
        // the session must be stopped.  frameId = None means "no frame context" — skip.
        if let Some(requested_frame_id) = args.frame_id {
            let session_guard = lock_or_recover(&self.session, "debug_adapter.session");
            match *session_guard {
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "evaluate".to_string(),
                        body: None,
                        message: Some("No debugger session".to_string()),
                    };
                }
                Some(ref session) => {
                    if session.state != DebugState::Stopped {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "evaluate".to_string(),
                            body: None,
                            message: Some(
                                "Cannot evaluate in frame context: session is not stopped"
                                    .to_string(),
                            ),
                        };
                    }
                    let frame_found =
                        session.stack_frames.iter().any(|f| i64::from(f.id) == requested_frame_id);
                    if !frame_found {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "evaluate".to_string(),
                            body: None,
                            message: Some(format!(
                                "Frame not found: frameId {requested_frame_id} does not match any \
                                 current stack frame"
                            )),
                        };
                    }
                }
            }
        }

        let expression = &args.expression;

        // AC10.3: Get timeout configuration (5s default, 30s hard limit)
        let timeout_ms = Self::debugger_timeout_budget_ms(5000) as u32;

        // Send evaluation command to debugger
        let output_frame_markers = if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                // Frame debugger output so evaluate parsing only considers this request's output.
                let commands = vec![format!("x {expression}")];
                match self.send_framed_debugger_commands(stdin, &commands) {
                    Ok(markers) => Some(markers),
                    Err(error) => {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "evaluate".to_string(),
                            body: None,
                            message: Some(format!("Failed to send evaluate command: {error}")),
                        };
                    }
                }
            } else {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "evaluate".to_string(),
                    body: None,
                    message: Some("No debugger session active".to_string()),
                };
            }
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some(format!(
                    "Evaluate is unavailable for processId attach (PID {pid}) without an active debugger transport"
                )),
            };
        } else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some("No debugger session".to_string()),
            };
        };

        let framed_lines = output_frame_markers.as_ref().and_then(|(begin, end)| {
            self.capture_framed_debugger_output(begin, end, u64::from(timeout_ms))
        });

        if let Some(lines) = framed_lines.as_ref()
            && let Some(error_line) = Self::parse_evaluate_error_from_lines(lines)
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some(error_line),
            };
        }

        let parsed = if let Some(lines) = framed_lines.as_ref() {
            Self::parse_evaluate_result_from_lines(lines, expression, true)
        } else {
            self.parse_evaluate_result_from_output(expression)
        };

        let Some((result, result_type)) = parsed else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some(format!(
                    "evaluate timed out after {timeout_ms}ms while evaluating `{expression}`"
                )),
            };
        };

        let variables_reference =
            self.allocate_evaluate_result_ref(expression, &result, &result_type);
        let eval_body =
            EvaluateResponseBody { result, type_: Some(result_type), variables_reference };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "evaluate".to_string(),
            body: serde_json::to_value(&eval_body).ok(),
            message: None,
        }
    }

    /// Handle setExpression request
    ///
    /// Assigns a value to an arbitrary Perl l-value expression using the debugger.
    /// Similar to setVariable but accepts full expressions (e.g. `$hash{key}`,
    /// `$array[0]`, `$obj->{field}`) rather than just simple variable names.
    pub(super) fn handle_set_expression(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SetExpressionArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setExpression".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let expression = args.expression.trim().to_string();
        let value = args.value.trim().to_string();
        let expression = expression.as_str();
        let value = value.as_str();

        if expression.is_empty() {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some("Missing expression".to_string()),
            };
        }

        if value.is_empty() {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some("Missing value".to_string()),
            };
        }

        if expression.contains('\n')
            || expression.contains('\r')
            || value.contains('\n')
            || value.contains('\r')
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some("Expression/value cannot contain newlines".to_string()),
            };
        }

        // Validate the VALUE with SafeEvaluator (the value is what gets evaluated)
        let evaluator = SafeEvaluator::new();
        if let Err(error) = evaluator.validate(value) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some(format!("Unsafe value for setExpression: {error}")),
            };
        }

        let output_frame_markers = if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let commands = vec![format!("p {expression} = {value}"), format!("p {expression}")];
                match self.send_framed_debugger_commands(stdin, &commands) {
                    Ok(markers) => Some(markers),
                    Err(error) => {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "setExpression".to_string(),
                            body: None,
                            message: Some(format!("Failed to send setExpression command: {error}")),
                        };
                    }
                }
            } else {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "setExpression".to_string(),
                    body: None,
                    message: Some("No debugger session active".to_string()),
                };
            }
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some(format!(
                    "setExpression is unavailable for processId attach (PID {pid}) without an active debugger transport"
                )),
            };
        } else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some("No debugger session".to_string()),
            };
        };

        let parsed = output_frame_markers
            .as_ref()
            .and_then(|(begin, end)| {
                self.capture_framed_debugger_output(begin, end, DEBUGGER_QUERY_WAIT_MS * 8)
            })
            .and_then(|lines| Self::parse_evaluate_result_from_lines(&lines, "", true));

        let Some((rendered_value, rendered_type)) = parsed else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setExpression".to_string(),
                body: None,
                message: Some(format!(
                    "setExpression read-back for `{expression}` produced no parseable output"
                )),
            };
        };

        let variables_reference =
            self.allocate_evaluate_result_ref(expression, &rendered_value, &rendered_type);
        let body = SetExpressionResponseBody {
            value: rendered_value,
            type_: Some(rendered_type),
            variables_reference,
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setExpression".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    /// Handle completions request — provides Perl keyword completions in the debug console.
    pub(super) fn handle_completions(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: CompletionsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "completions".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let byte_offset = (args.column.max(0) as usize).min(args.text.len());
        // Clamp to a valid UTF-8 char boundary to avoid panics on multi-byte input.
        let mut column = byte_offset;
        while column > 0 && !args.text.is_char_boundary(column) {
            column -= 1;
        }
        let prefix = &args.text[..column];

        // Find the last word boundary to get the completion stem.
        // rfind returns a byte position; we advance past the matched char (which may be multi-byte).
        let stem = prefix
            .rmatch_indices(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .map(|(pos, matched)| &prefix[pos + matched.len()..])
            .unwrap_or(prefix);

        let mut targets: Vec<CompletionItem> = DAP_COMPLETION_KEYWORDS
            .iter()
            .filter(|kw| stem.is_empty() || kw.starts_with(stem))
            .map(|kw| CompletionItem {
                label: (*kw).to_string(),
                type_: Some("keyword".to_string()),
                text: None,
                sort_text: None,
                detail: None,
                start: None,
                length: None,
            })
            .collect();

        // When a debug session is active, supplement with runtime data.
        let has_session = lock_or_recover(&self.session, "debug_adapter.session").is_some();

        let runtime_start = targets.len();
        if has_session {
            // Add variable names from cached session scope.
            {
                let session_guard = lock_or_recover(&self.session, "debug_adapter.session");
                if let Some(ref session) = *session_guard {
                    let mut seen = std::collections::HashSet::new();
                    for var in session.variable_cache.all_variables() {
                        if (stem.is_empty() || var.name.starts_with(stem))
                            && seen.insert(var.name.clone())
                        {
                            targets.push(CompletionItem {
                                label: var.name.clone(),
                                type_: Some("variable".to_string()),
                                text: None,
                                sort_text: None,
                                detail: None,
                                start: None,
                                length: None,
                            });
                        }
                    }
                }
            }

            // Add loaded module names from %INC.
            let modules = self.query_inc_entries();
            for (key, _path) in &modules {
                let name = module_path_to_name(key);
                if stem.is_empty() || name.starts_with(stem) {
                    targets.push(CompletionItem {
                        label: name,
                        type_: Some("module".to_string()),
                        text: None,
                        sort_text: None,
                        detail: None,
                        start: None,
                        length: None,
                    });
                }
            }

            // Sort runtime completions for deterministic output.
            targets[runtime_start..].sort_by(|a, b| a.label.cmp(&b.label));
        }

        let body = CompletionsResponseBody { targets };
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "completions".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    /// Determine whether a type name produced by the Perl debugger refers to a
    /// structured container (HASH, ARRAY, REF, blessed object, TIED).
    ///
    /// Returns `true` when the result can be further expanded via a `variables`
    /// request; `false` for scalars, integers, and plain strings.
    fn result_type_is_expandable(type_name: &str) -> bool {
        matches!(type_name, "HASH" | "ARRAY" | "REF" | "OBJECT" | "TIED")
            || type_name.contains("HASH")
            || type_name.contains("ARRAY")
    }

    /// Allocate a `variablesReference` for a structured evaluate/setExpression/setVariable
    /// result and cache a placeholder entry so a follow-up `variables` request can expand it.
    ///
    /// Returns `0` when the result type is not expandable or when no active session
    /// is available (the ref cannot be served without a cache).
    ///
    /// ## Ref-range non-collision guarantee
    ///
    /// Scope refs are `frame_id * 10 + scope_type` (scope_type in {1, 2, 3}).
    /// A frame_id of 5_000 already produces scope ref 50_001, which would collide
    /// with a naive 50_000 base offset.  This function uses a 1_000_000 base
    /// instead: `frame_id` would need to reach 100_000 to collide, which is
    /// impossible in practice and `saturating_mul` caps well before i32::MAX.
    pub(super) fn allocate_evaluate_result_ref(
        &self,
        expression: &str,
        result: &str,
        result_type: &str,
    ) -> i64 {
        if !Self::result_type_is_expandable(result_type) {
            return 0;
        }
        if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.allocate_evaluate_result_ref")
        {
            let raw_counter = self.debugger_output_marker.fetch_add(1, Ordering::Relaxed);
            let counter = Self::i64_to_i32_saturating(raw_counter as i64);
            let eval_ref = crate::debug_adapter::var_ref::VariableReference::EvalResult { counter }
                .encode()
                .unwrap_or(0);
            let placeholder = Variable {
                name: expression.to_string(),
                value: result.to_string(),
                type_: Some(result_type.to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            };
            session.variable_cache.upsert(
                eval_ref,
                VariableCacheKind::EvaluateResult,
                vec![placeholder],
            );
            i64::from(eval_ref)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod evaluate_allocation_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // result_type_is_expandable — cover all arms including contains-HASH/ARRAY
    // -----------------------------------------------------------------------

    #[test]
    fn expandable_exact_matches() {
        for ty in ["HASH", "ARRAY", "REF", "OBJECT", "TIED"] {
            assert!(DebugAdapter::result_type_is_expandable(ty), "{ty} should be expandable");
        }
    }

    #[test]
    fn expandable_contains_hash() {
        // Blessed objects often have type strings like "SomeClass=HASH(0x...)"
        assert!(DebugAdapter::result_type_is_expandable("SomeClass=HASH(0x1234)"));
        assert!(DebugAdapter::result_type_is_expandable("My::Module=HASH"));
    }

    #[test]
    fn expandable_contains_array() {
        assert!(DebugAdapter::result_type_is_expandable("SomeClass=ARRAY(0x1234)"));
        assert!(DebugAdapter::result_type_is_expandable("Tied=ARRAY"));
    }

    #[test]
    fn not_expandable_scalar_types() {
        for ty in ["SCALAR", "INTEGER", "FLOAT", "STRING", "UNDEF", "CODE", "IO"] {
            assert!(!DebugAdapter::result_type_is_expandable(ty), "{ty} should not be expandable");
        }
    }

    // -----------------------------------------------------------------------
    // allocate_evaluate_result_ref — no-session path (else { 0 }) is the key
    // changed line that needs coverage.  When no session is present, even an
    // expandable type must return 0.
    // -----------------------------------------------------------------------

    #[test]
    fn allocate_returns_zero_with_no_session_and_expandable_type() {
        // Without a session the else-branch returns 0 even for expandable types.
        // This covers the `else { 0 }` arm of allocate_evaluate_result_ref.
        let adapter = DebugAdapter::new();
        let ref_val = adapter.allocate_evaluate_result_ref("$h", "HASH(0x1234)", "HASH");
        assert_eq!(
            ref_val, 0,
            "allocate_evaluate_result_ref must return 0 when no session is present"
        );
    }

    #[test]
    fn allocate_returns_zero_for_non_expandable_type() {
        // Covers the early-return `if !Self::result_type_is_expandable` arm.
        let adapter = DebugAdapter::new();
        let ref_val = adapter.allocate_evaluate_result_ref("$x", "42", "SCALAR");
        assert_eq!(
            ref_val, 0,
            "allocate_evaluate_result_ref must return 0 for non-expandable scalar type"
        );
    }

    #[test]
    fn allocate_returns_zero_for_ref_type_no_session() {
        // Cover REF type (not just HASH/ARRAY) through the no-session path.
        let adapter = DebugAdapter::new();
        let ref_val = adapter.allocate_evaluate_result_ref("\\$x", "REF(0xabcd)", "REF");
        assert_eq!(ref_val, 0, "REF type with no session must return 0");
    }

    #[test]
    fn allocate_returns_zero_for_blessed_hash_no_session() {
        // Cover the contains-HASH arm of result_type_is_expandable through the
        // no-session path of allocate_evaluate_result_ref.
        let adapter = DebugAdapter::new();
        let ref_val =
            adapter.allocate_evaluate_result_ref("$obj", "SomeClass=HASH(0x1)", "SomeClass=HASH");
        assert_eq!(ref_val, 0, "blessed HASH type with no session must return 0");
    }

    #[test]
    fn allocate_returns_zero_for_blessed_array_no_session() {
        // Cover the contains-ARRAY arm of result_type_is_expandable through the
        // no-session path of allocate_evaluate_result_ref.
        let adapter = DebugAdapter::new();
        let ref_val =
            adapter.allocate_evaluate_result_ref("$arr_obj", "Iter=ARRAY(0x1)", "Iter=ARRAY");
        assert_eq!(ref_val, 0, "blessed ARRAY type with no session must return 0");
    }

    // -----------------------------------------------------------------------
    // allocate_evaluate_result_ref — session-present path (lines 511-530).
    // When a live session is present and the type is expandable, allocate a
    // non-zero variablesReference in the 1_000_000+ range and cache the entry.
    // -----------------------------------------------------------------------

    #[test]
    fn allocate_returns_nonzero_with_live_session_and_hash_type() {
        // Covers the session-present branch: raw_counter fetch_add, eval_ref
        // computation (1_000_000 base), Variable construction, cache upsert.
        let adapter = DebugAdapter::new();
        adapter.seed_session_for_test();

        let ref_val = adapter.allocate_evaluate_result_ref("$h", "HASH(0x1234)", "HASH");

        assert!(
            ref_val >= 1_000_000,
            "variablesReference must be in the 1_000_000+ range to avoid scope-ref collision; got {ref_val}"
        );
        assert_ne!(ref_val, 0, "session-present HASH must return non-zero variablesReference");
    }

    #[test]
    fn allocate_returns_nonzero_with_live_session_and_array_type() {
        // Same session-present coverage path for ARRAY type.
        let adapter = DebugAdapter::new();
        adapter.seed_session_for_test();

        let ref_val = adapter.allocate_evaluate_result_ref("@arr", "ARRAY(0xabcd)", "ARRAY");

        assert!(ref_val >= 1_000_000, "ARRAY ref must be in 1_000_000+ range; got {ref_val}");
        assert_ne!(ref_val, 0, "session-present ARRAY must return non-zero variablesReference");
    }

    #[test]
    fn allocate_refs_are_monotonically_increasing() {
        // Verifies successive allocations increment the counter so refs are
        // unique — covers the fetch_add path through multiple calls.
        let adapter = DebugAdapter::new();
        adapter.seed_session_for_test();

        let ref1 = adapter.allocate_evaluate_result_ref("$a", "HASH(0x1)", "HASH");
        let ref2 = adapter.allocate_evaluate_result_ref("$b", "HASH(0x2)", "HASH");

        assert!(ref1 >= 1_000_000, "first ref must be in 1_000_000+ range; got {ref1}");
        assert!(ref2 > ref1, "second ref must be greater than first; got ref1={ref1}, ref2={ref2}");
    }

    #[test]
    fn allocate_caches_placeholder_variable_in_session() {
        // Verifies the allocated ref is retrievable from the session cache via
        // get_page — proving the upsert call ran and the placeholder was stored.
        let adapter = DebugAdapter::new();
        adapter.seed_session_for_test();

        let expression = "$my_hash";
        let result_val = "HASH(0x5678)";
        let result_type = "HASH";
        let ref_val = adapter.allocate_evaluate_result_ref(expression, result_val, result_type);

        assert!(ref_val >= 1_000_000, "ref must be in 1_000_000+ range; got {ref_val}");
        let ref_i32 = ref_val as i32;

        // Read the placeholder back from the session variable_cache.
        let mut session_guard =
            lock_or_recover(&adapter.session, "test_allocate_caches_placeholder");
        let vars = session_guard.as_mut().and_then(|s| s.variable_cache.get_page(ref_i32, 0, 10));
        assert!(vars.is_some(), "cache must contain the placeholder variable for ref {ref_val}");
        let vars = vars.unwrap_or_default();
        assert_eq!(vars.len(), 1, "exactly one placeholder variable expected; got {}", vars.len());
        assert_eq!(vars[0].name, expression, "placeholder name must match expression");
        assert_eq!(vars[0].value, result_val, "placeholder value must match result");
    }
}
