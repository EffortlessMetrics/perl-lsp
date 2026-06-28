use super::*;

impl DebugAdapter {
    /// Handle dataBreakpointInfo request — check if a variable can be watched.
    pub(in crate::debug_adapter) fn handle_data_breakpoint_info(
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
    pub(in crate::debug_adapter) fn handle_set_data_breakpoints(
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
