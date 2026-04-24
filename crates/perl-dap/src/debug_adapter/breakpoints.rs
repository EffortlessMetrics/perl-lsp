//! Breakpoint management: set, clear, function breakpoints, exception breakpoints, data breakpoints.

use super::*;

impl DebugAdapter {
    /// Handle setBreakpoints request
    pub(super) fn handle_set_breakpoints(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let Some(args_value) = arguments else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setBreakpoints".to_string(),
                body: None,
                message: Some("Missing arguments".to_string()),
            };
        };

        let args: crate::protocol::SetBreakpointsArguments =
            match serde_json::from_value(args_value) {
                Ok(a) => a,
                Err(e) => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setBreakpoints".to_string(),
                        body: None,
                        message: Some(format!("Invalid arguments: {}", e)),
                    };
                }
            };

        // Snapshot old breakpoints for this file before replacing them,
        // so we can clear only per-file breakpoints instead of global `B *`.
        let old_breakpoints = if let Some(ref source_path) = args.source.path {
            self.breakpoints.get_breakpoints(source_path)
        } else {
            Vec::new()
        };

        // AC7: AST-based breakpoint validation via BreakpointStore
        let verified_breakpoints = self.breakpoints.set_breakpoints(&args);

        // If a session is active, also sync the breakpoints to the Perl debugger
        if let Ok(mut guard) = self.session.lock()
            && let Some(ref mut session) = *guard
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let mut sync_commands = String::new();

                // Clear only the old breakpoints for this specific file
                for old_bp in &old_breakpoints {
                    if old_bp.verified {
                        sync_commands.push_str(&format!("B {}\n", old_bp.line));
                    }
                }

                // Set new breakpoints that were successfully verified
                if let Some(source_path) = args.source.path.as_deref() {
                    let active_breakpoints = self.breakpoints.get_breakpoints(source_path);
                    for record in active_breakpoints {
                        if record.verified {
                            if let Some(cond) = record.condition {
                                sync_commands.push_str(&format!("b {} {}\n", record.line, cond));
                            } else {
                                sync_commands.push_str(&format!("b {}\n", record.line));
                            }
                        }
                    }
                }

                if !sync_commands.is_empty() {
                    let _ = stdin.write_all(sync_commands.as_bytes());
                    let _ = stdin.flush();
                }
            }
        }

        // Keep function breakpoints active after line-breakpoint synchronization.
        self.apply_stored_function_breakpoints();

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setBreakpoints".to_string(),
            body: Some(json!({
                "breakpoints": verified_breakpoints
            })),
            message: None,
        }
    }

    /// Handle setFunctionBreakpoints request.
    ///
    /// Uses replace semantics and best-effort synchronization to the running debugger.
    pub(super) fn handle_set_function_breakpoints(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SetFunctionBreakpointsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setFunctionBreakpoints".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let requested = args.breakpoints;

        let mut validated_names = Vec::with_capacity(requested.len());
        let mut response_breakpoints = Vec::with_capacity(requested.len());

        for entry in requested {
            let name = entry.name.trim().to_string();

            let id = {
                let mut next = lock_or_recover(
                    &self.next_function_breakpoint_id,
                    "debug_adapter.next_function_breakpoint_id",
                );
                let id = *next;
                *next += 1;
                id
            };

            let invalid_reason = if name.is_empty() {
                Some("Function breakpoint name is required".to_string())
            } else if name.contains('\n') || name.contains('\r') {
                Some("Function breakpoint name cannot contain newlines".to_string())
            } else if !is_valid_function_breakpoint_name(&name) {
                Some(format!(
                    "Invalid function breakpoint name `{name}` (expected package-qualified Perl symbol)"
                ))
            } else {
                None
            };

            if let Some(reason) = invalid_reason {
                response_breakpoints.push(json!({
                    "id": id,
                    "verified": false,
                    "message": reason
                }));
                continue;
            }

            validated_names.push(name.clone());
            response_breakpoints.push(json!({
                "id": id,
                "verified": true
            }));
        }

        // DAP replace semantics: overwrite existing function breakpoints.
        if let Ok(mut stored) = self.function_breakpoints.lock() {
            *stored = validated_names;
        }

        // Best-effort apply to currently running session as well.
        self.apply_stored_function_breakpoints();

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setFunctionBreakpoints".to_string(),
            body: Some(json!({ "breakpoints": response_breakpoints })),
            message: None,
        }
    }

    /// Handle setExceptionBreakpoints request.
    ///
    /// Supports `die`/uncaught exception breaks via output classification in the
    /// debugger reader thread.
    pub(super) fn handle_set_exception_breakpoints(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let mut break_on_die = false;
        let mut break_on_warn = false;
        let supports_die = catalog_has_feature("dap.exceptions.die");
        let supports_warn = catalog_has_feature("dap.exceptions.warn");

        if let Some(args) = arguments
            .and_then(|v| serde_json::from_value::<SetExceptionBreakpointsArguments>(v).ok())
        {
            let matches_filter = |id: &str| -> (bool, bool) {
                let all = id.eq_ignore_ascii_case("all");
                let die = supports_die && (id.eq_ignore_ascii_case("die") || all);
                let warn = supports_warn && (id.eq_ignore_ascii_case("warn") || all);
                (die, warn)
            };

            for filter in &args.filters {
                let (die, warn) = matches_filter(filter);
                break_on_die |= die;
                break_on_warn |= warn;
            }

            if let Some(filter_options) = args.filter_options {
                for entry in &filter_options {
                    let (die, warn) = matches_filter(&entry.filter_id);
                    break_on_die |= die;
                    break_on_warn |= warn;
                }
            }
        }

        if let Ok(mut guard) = self.exception_break_on_die.lock() {
            *guard = break_on_die;
        }
        if let Ok(mut guard) = self.exception_break_on_warn.lock() {
            *guard = break_on_warn;
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setExceptionBreakpoints".to_string(),
            body: Some(json!({ "breakpoints": [] })),
            message: None,
        }
    }

    pub(super) fn handle_breakpoint_locations(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: BreakpointLocationsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "breakpointLocations".to_string(),
                        body: None,
                        message: Some("Missing or invalid arguments".to_string()),
                    };
                }
            };

        let source_path = match args.source.path {
            Some(ref p) => p.clone(),
            None => {
                let body = BreakpointLocationsResponseBody { breakpoints: Vec::new() };
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "breakpointLocations".to_string(),
                    body: serde_json::to_value(&body).ok(),
                    message: None,
                };
            }
        };

        // Validate path against workspace root to prevent path traversal
        let validated_path = match self.validate_source_path(&source_path) {
            Ok(p) => p,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "breakpointLocations".to_string(),
                    body: None,
                    message: Some(e),
                };
            }
        };

        let content = match std::fs::read_to_string(&validated_path) {
            Ok(c) => c,
            Err(_) => {
                let body = BreakpointLocationsResponseBody { breakpoints: Vec::new() };
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "breakpointLocations".to_string(),
                    body: serde_json::to_value(&body).ok(),
                    message: None,
                };
            }
        };

        let mut breakpoints = Vec::new();
        let end_line = args.end_line.unwrap_or(args.line);

        if let Ok(validator) = AstBreakpointValidator::new(&content) {
            for line in args.line..=end_line {
                if self.cancel_requested.load(Ordering::Acquire) {
                    self.cancel_requested.store(false, Ordering::Release);
                    break;
                }
                if validator.is_executable_line(line) {
                    breakpoints.push(BreakpointLocation {
                        line,
                        column: None,
                        end_line: None,
                        end_column: None,
                    });
                }
            }
        }

        let body = BreakpointLocationsResponseBody { breakpoints };
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "breakpointLocations".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    /// Handle dataBreakpointInfo request — check if a variable can be watched.
    pub(super) fn handle_data_breakpoint_info(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: DataBreakpointInfoArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "dataBreakpointInfo".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        let body = if is_valid_set_variable_name(&args.name) {
            DataBreakpointInfoResponseBody {
                data_id: Some(args.name.clone()),
                description: format!("Watch `{}` for write access", args.name),
                access_types: Some(vec!["write".to_string()]),
            }
        } else {
            DataBreakpointInfoResponseBody {
                data_id: None,
                description: "Cannot watch this expression".to_string(),
                access_types: None,
            }
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "dataBreakpointInfo".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    /// Handle setDataBreakpoints request — set watchpoints via Perl debugger `w` command.
    pub(super) fn handle_set_data_breakpoints(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SetDataBreakpointsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "setDataBreakpoints".to_string(),
                        body: None,
                        message: Some("Missing arguments".to_string()),
                    };
                }
            };

        // Store the data breakpoints.
        {
            let mut store =
                lock_or_recover(&self.data_breakpoints, "debug_adapter.data_breakpoints");
            *store = args
                .breakpoints
                .iter()
                .map(|bp| DataBreakpointRecord {
                    data_id: bp.data_id.clone(),
                    access_type: bp.access_type.clone(),
                    condition: bp.condition.clone(),
                })
                .collect();
        }

        // If session active, set watchpoints via the debugger.
        {
            let mut session_guard = lock_or_recover(&self.session, "debug_adapter.session");
            if let Some(ref mut session) = *session_guard {
                if let Some(stdin) = session.process.stdin.as_mut() {
                    // Build commands: clear all watchpoints, then set each.
                    let mut commands = vec!["W *".to_string()];
                    for bp in &args.breakpoints {
                        commands.push(format!("w {}", bp.data_id));
                    }
                    // Fire-and-forget: we don't need the output.
                    let _ = self.send_framed_debugger_commands(stdin, &commands);
                }
            }
            // Session guard dropped here.
        }

        // Build response breakpoints — one per input.
        let response_breakpoints: Vec<crate::protocol::Breakpoint> = args
            .breakpoints
            .iter()
            .enumerate()
            .map(|(idx, _bp)| crate::protocol::Breakpoint {
                id: (idx as i64) + 1,
                verified: true,
                line: 0,
                column: None,
                message: None,
            })
            .collect();

        let body = SetDataBreakpointsResponseBody { breakpoints: response_breakpoints };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "setDataBreakpoints".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }
}
