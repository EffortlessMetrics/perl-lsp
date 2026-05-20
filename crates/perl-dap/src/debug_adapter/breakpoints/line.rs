use super::*;

impl DebugAdapter {
    /// Handle setBreakpoints request
    pub(in crate::debug_adapter) fn handle_set_breakpoints(
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
        let new_breakpoint_records = if let Some(ref source_path) = args.source.path {
            self.breakpoints.get_breakpoints(source_path)
        } else {
            Vec::new()
        };
        let condition_by_id: HashMap<i64, Option<String>> = new_breakpoint_records
            .into_iter()
            .map(|record| (record.id, record.condition))
            .collect();

        // If a session is active, also sync the breakpoints to the Perl debugger
        if let Ok(mut guard) = self.session.lock()
            && let Some(ref mut session) = *guard
        {
            if let Some(stdin) = session.process.stdin.as_mut() {
                let mut command_batch = String::new();

                // Clear only the old breakpoints for this specific file
                for old_bp in &old_breakpoints {
                    if old_bp.verified {
                        command_batch.push_str(&format!("B {}\n", old_bp.line));
                    }
                }

                // Set new breakpoints that were successfully verified
                for bp in &verified_breakpoints {
                    if bp.verified {
                        // Retrieve original condition (if present) from records produced by this call.
                        let cmd = if let Some(Some(cond)) = condition_by_id.get(&bp.id) {
                            format!("b {} {}\n", bp.line, cond)
                        } else {
                            format!("b {}\n", bp.line)
                        };
                        command_batch.push_str(&cmd);
                    }
                }

                if !command_batch.is_empty() {
                    let _ = stdin.write_all(command_batch.as_bytes());
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
}
