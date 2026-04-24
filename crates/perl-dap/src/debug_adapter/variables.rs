//! Variable inspection: variable display, scope variables, set variable.

use super::*;

impl DebugAdapter {
    pub(super) const MAX_VARIABLE_CACHE_ENTRIES: usize = 1024;

    fn is_scope_root_reference(variables_ref: i32) -> bool {
        matches!(variables_ref % 10, 1..=3)
    }

    fn slice_cached_variables(variables: &[Variable], start: usize, count: usize) -> Vec<Variable> {
        variables.iter().skip(start).take(count).cloned().collect()
    }

    /// Handle variables request
    pub(super) fn handle_variables(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: VariablesArguments = match arguments.and_then(|v| serde_json::from_value(v).ok())
        {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "variables".to_string(),
                    body: None,
                    message: Some("Missing arguments".to_string()),
                };
            }
        };

        if args.start.is_some_and(|start| start < 0) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "variables".to_string(),
                body: None,
                message: Some("Invalid start: must be >= 0".to_string()),
            };
        }

        if args.count.is_some_and(|count| count < 0) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "variables".to_string(),
                body: None,
                message: Some("Invalid count: must be >= 0".to_string()),
            };
        }

        let variables_ref = args.variables_reference as i32;
        let start = args.start.unwrap_or(0) as usize;
        let count = args.count.map(|v| v as usize).unwrap_or(256).clamp(1, 1024);
        if variables_ref == 0 {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "variables".to_string(),
                body: None,
                message: Some("Missing variablesReference".to_string()),
            };
        }

        // Child expansion path: serve from child cache with local slicing.
        if !Self::is_scope_root_reference(variables_ref) {
            if let Some(ref session) = *lock_or_recover(&self.session, "debug_adapter.session")
                && let Some(children) = session.child_variables.get(&variables_ref)
            {
                let variables = Self::slice_cached_variables(children, start, count);
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "variables".to_string(),
                    body: Some(json!({ "variables": variables })),
                    message: None,
                };
            }

            return DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "variables".to_string(),
                body: Some(json!({ "variables": Vec::<Variable>::new() })),
                message: None,
            };
        }

        // Scope root path: use root cache, otherwise parse once then cache roots + children.
        let parsed_from_output;
        let mut parsed_child_cache = HashMap::new();
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session") {
            if let Some(cached) = session.root_variables.get(&variables_ref) {
                let variables = Self::slice_cached_variables(cached, start, count);
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "variables".to_string(),
                    body: Some(json!({ "variables": variables })),
                    message: None,
                };
            } else {
                let mut framed_scope_lines = None;

                // Request fresh scope output from Perl debugger for scope roots only.
                let frame_id = variables_ref / 10;
                match variables_ref % 10 {
                    1 => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} .", frame_id)];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    tracing::warn!(%error, "Failed to send framed variables command, falling back");
                                    let cmd = format!("V {} .\n", frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    2 => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} ::", frame_id)];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    tracing::warn!(%error, "Failed to send framed variables command, falling back");
                                    let cmd = format!("V {} ::\n", frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    3 => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} *", frame_id)];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    tracing::warn!(%error, "Failed to send framed variables command, falling back");
                                    let cmd = format!("V {} *\n", frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    _ => {}
                }

                let (vars, child_cache) = if let Some(lines) = framed_scope_lines.as_ref() {
                    let (framed_vars, framed_child_cache) =
                        Self::parse_scope_variables_from_lines(
                            lines,
                            variables_ref,
                            0,
                            Self::MAX_VARIABLE_CACHE_ENTRIES,
                        );
                    if framed_vars.is_empty() {
                        Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                        self.parse_scope_variables_from_output(
                            variables_ref,
                            0,
                            Self::MAX_VARIABLE_CACHE_ENTRIES,
                        )
                    } else {
                        (framed_vars, framed_child_cache)
                    }
                } else {
                    Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                    self.parse_scope_variables_from_output(
                        variables_ref,
                        0,
                        Self::MAX_VARIABLE_CACHE_ENTRIES,
                    )
                };

                parsed_from_output = vars;
                parsed_child_cache = child_cache;
            }
        } else {
            let (vars, _child_cache) =
                self.parse_scope_variables_from_output(
                    variables_ref,
                    0,
                    Self::MAX_VARIABLE_CACHE_ENTRIES,
                );
            parsed_from_output = vars;
        }

        let uncached_variables = if parsed_from_output.is_empty() {
            Self::fallback_scope_variables(variables_ref, start, count)
        } else {
            Self::slice_cached_variables(&parsed_from_output, start, count)
        };

        // Cache parsed variables and generated child references for expansion requests.
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && !parsed_from_output.is_empty()
        {
            session.root_variables.insert(variables_ref, parsed_from_output);
            for (reference, children) in parsed_child_cache {
                session.child_variables.insert(reference, children);
            }
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "variables".to_string(),
            body: Some(json!({
                "variables": uncached_variables
            })),
            message: None,
        }
    }

    /// Handle setVariable request
    pub(super) fn handle_set_variable(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SetVariableArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setVariable".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let variables_ref = args.variables_reference;
        if variables_ref <= 0 {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Missing variablesReference".to_string()),
            };
        }

        let name = args.name.trim().to_string();
        let value = args.value.trim().to_string();
        let name = name.as_str();
        let value = value.as_str();

        if name.is_empty() {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Missing variable name".to_string()),
            };
        }

        if value.is_empty() {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Missing variable value".to_string()),
            };
        }

        if name.contains('\n')
            || name.contains('\r')
            || value.contains('\n')
            || value.contains('\r')
        {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some("Variable name/value cannot contain newlines".to_string()),
            };
        }

        if !is_valid_set_variable_name(name) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some(format!(
                    "Invalid variable name `{name}` for setVariable (expected Perl sigil-prefixed variable)"
                )),
            };
        }

        let output_frame_markers = if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                // Frame assignment + read-back so output parsing is deterministic.
                let commands = vec![format!("p {name} = {value}"), format!("p {name}")];
                match self.send_framed_debugger_commands(stdin, &commands) {
                    Ok(markers) => Some(markers),
                    Err(error) => {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "setVariable".to_string(),
                            body: None,
                            message: Some(format!("Failed to send setVariable command: {error}")),
                        };
                    }
                }
            } else {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "setVariable".to_string(),
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
                command: "setVariable".to_string(),
                body: None,
                message: Some(format!(
                    "setVariable is unavailable for processId attach (PID {pid}) without an active debugger transport"
                )),
            };
        } else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
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
                command: "setVariable".to_string(),
                body: None,
                message: Some(format!(
                    "setVariable read-back for `{name}` produced no parseable output"
                )),
            };
        };

        let set_var_body = SetVariableResponseBody {
            value: rendered_value,
            type_: Some(rendered_type),
            variables_reference: 0,
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setVariable".to_string(),
            body: serde_json::to_value(&set_var_body).ok(),
            message: None,
        }
    }
}
