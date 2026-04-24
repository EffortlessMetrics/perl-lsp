//! Variable inspection: variable display, scope variables, set variable.

use super::*;

impl DebugAdapter {
    const VARIABLE_CACHE_PARSE_LIMIT: usize = 256;

    fn slice_cached_variables(variables: &[Variable], start: usize, count: usize) -> Vec<Variable> {
        variables.iter().skip(start).take(count).cloned().collect()
    }

    fn request_scope_variables_lines(
        &self,
        session: &mut DebugSession,
        variables_ref: i32,
    ) -> Option<Vec<String>> {
        let frame_id = variables_ref / 10;
        let scope_selector = match variables_ref % 10 {
            1 => ".",
            2 => "::",
            3 => "*",
            _ => return None,
        };
        let stdin = session.process.stdin.as_mut()?;
        let command = format!("V {frame_id} {scope_selector}");
        match self.send_framed_debugger_commands(stdin, &[command]) {
            Ok((begin, end)) => {
                self.capture_framed_debugger_output(&begin, &end, DEBUGGER_QUERY_WAIT_MS * 8)
            }
            Err(error) => {
                tracing::warn!(%error, "Failed to send framed variables command, falling back");
                let fallback_command = format!("V {frame_id} {scope_selector}\n");
                let _ = stdin.write_all(fallback_command.as_bytes());
                let _ = stdin.flush();
                None
            }
        }
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

        let mut cached_response = None;
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session") {
            if let Some(vars) = session.variables.get(&variables_ref) {
                cached_response = Some(Self::slice_cached_variables(vars, start, count));
            }
        }

        if let Some(variables) = cached_response {
            return DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "variables".to_string(),
                body: Some(json!({
                    "variables": variables
                })),
                message: None,
            };
        }

        // Parse and cache full root/child vectors so repeated and paged expansions are local.
        let uncached_variables = if let Some(ref mut session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            let framed_scope_lines = self.request_scope_variables_lines(session, variables_ref);

            let (parsed_full_variables, parsed_child_cache) =
                if let Some(lines) = framed_scope_lines.as_ref() {
                    let (framed_vars, framed_child_cache) = Self::parse_scope_variables_from_lines(
                        lines,
                        variables_ref,
                        0,
                        Self::VARIABLE_CACHE_PARSE_LIMIT,
                    );
                    if framed_vars.is_empty() {
                        Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                        self.parse_scope_variables_from_output(
                            variables_ref,
                            0,
                            Self::VARIABLE_CACHE_PARSE_LIMIT,
                        )
                    } else {
                        (framed_vars, framed_child_cache)
                    }
                } else {
                    Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                    self.parse_scope_variables_from_output(
                        variables_ref,
                        0,
                        Self::VARIABLE_CACHE_PARSE_LIMIT,
                    )
                };

            let full_variables = if parsed_full_variables.is_empty() {
                if variables_ref % 10 == 1 || variables_ref % 10 == 2 || variables_ref % 10 == 3 {
                    Self::fallback_scope_variables(
                        variables_ref,
                        0,
                        Self::VARIABLE_CACHE_PARSE_LIMIT,
                    )
                } else {
                    Vec::new()
                }
            } else {
                parsed_full_variables
            };

            session.variables.insert(variables_ref, full_variables.clone());
            for (child_ref, children) in parsed_child_cache {
                session.variables.insert(child_ref, children);
            }
            Self::slice_cached_variables(&full_variables, start, count)
        } else {
            let (parsed_full_variables, _child_cache) = self.parse_scope_variables_from_output(
                variables_ref,
                0,
                Self::VARIABLE_CACHE_PARSE_LIMIT,
            );
            let full_variables = if parsed_full_variables.is_empty() {
                if variables_ref % 10 == 1 || variables_ref % 10 == 2 || variables_ref % 10 == 3 {
                    Self::fallback_scope_variables(
                        variables_ref,
                        0,
                        Self::VARIABLE_CACHE_PARSE_LIMIT,
                    )
                } else {
                    Vec::new()
                }
            } else {
                parsed_full_variables
            };
            Self::slice_cached_variables(&full_variables, start, count)
        };

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
