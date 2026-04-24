//! REPL and expression evaluation: evaluate, set expression, completions.

use super::*;

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
                let evaluator = SafeEvaluator::new();
                if let Err(error) = evaluator.validate(expression) {
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

        let parsed = framed_lines
            .as_ref()
            .and_then(|lines| Self::parse_evaluate_result_from_lines(lines, expression, true))
            .or_else(|| self.parse_evaluate_result_from_output(expression));

        let Some((result, result_type)) = parsed else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "evaluate".to_string(),
                body: None,
                message: Some(format!("evaluate timed out after {timeout_ms}ms")),
            };
        };

        let eval_body =
            EvaluateResponseBody { result, type_: Some(result_type), variables_reference: 0 };

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

        let body = SetExpressionResponseBody {
            value: rendered_value,
            type_: Some(rendered_type),
            variables_reference: 0,
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
                    for vars in session
                        .root_variables
                        .values()
                        .chain(session.child_variables.values())
                    {
                        for var in vars {
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
}
