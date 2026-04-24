//! Execution control: continue, next, step in, step out, pause, goto, cancel.

use super::*;
use std::sync::LazyLock;

static STEP_IN_TARGET_CALL_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(\w[\w:]*)\s*\(").ok());

impl DebugAdapter {
    /// Handle continue request
    pub(super) fn handle_continue(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<ContinueArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        let mut thread_id = 1;
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"c\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::Continue;
            session.variables.clear();
            session.stack_trace_cache = None;
            thread_id = session.thread_id;
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            let _ = self.send_continue_signal(pid);
            thread_id = Self::i64_to_i32_saturating(i64::from(pid));
        }

        // AC9.4: Proper DAP event emission: continued
        self.send_event(
            "continued",
            Some(json!({
                "threadId": thread_id,
                "allThreadsContinued": true
            })),
        );

        let continue_body = ContinueResponseBody { all_threads_continued: true };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "continue".to_string(),
            body: serde_json::to_value(&continue_body).ok(),
            message: None,
        }
    }

    /// Handle next request
    pub(super) fn handle_next(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<NextArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"n\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::Next;
            session.variables.clear();
            session.stack_trace_cache = None;
            let t_id = session.thread_id;
            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "next".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle stepIn request
    pub(super) fn handle_step_in(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<StepInArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"s\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::StepIn;
            session.variables.clear();
            session.stack_trace_cache = None;
            let t_id = session.thread_id;
            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "stepIn".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle stepOut request
    pub(super) fn handle_step_out(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<StepOutArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            let _ = stdin.write_all(b"r\n");
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::StepOut;
            session.variables.clear();
            session.stack_trace_cache = None;
            let t_id = session.thread_id;
            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "stepOut".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle pause request
    pub(super) fn handle_pause(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<PauseArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());
        let success = if let Some(ref session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            let pid = session.process.id();
            self.send_interrupt_signal(pid)
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            self.send_interrupt_signal(pid)
        } else {
            tracing::warn!("No active debug session to pause");
            false
        };

        DapMessage::Response {
            seq,
            request_seq,
            success,
            command: "pause".to_string(),
            body: None,
            message: if !success { Some("Failed to pause debugger".to_string()) } else { None },
        }
    }

    pub(super) fn handle_goto_targets(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: GotoTargetsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "gotoTargets".to_string(),
                        body: None,
                        message: Some("Missing or invalid arguments".to_string()),
                    };
                }
            };

        let source_path = match args.source.path {
            Some(ref p) => p.clone(),
            None => {
                let body = GotoTargetsResponseBody { targets: Vec::new() };
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "gotoTargets".to_string(),
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
                    command: "gotoTargets".to_string(),
                    body: None,
                    message: Some(e),
                };
            }
        };

        let content = match std::fs::read_to_string(&validated_path) {
            Ok(c) => c,
            Err(_) => {
                let body = GotoTargetsResponseBody { targets: Vec::new() };
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "gotoTargets".to_string(),
                    body: serde_json::to_value(&body).ok(),
                    message: None,
                };
            }
        };

        // Clear stale goto target mappings and build fresh ones
        let mut goto_map = lock_or_recover(&self.goto_targets, "debug_adapter.goto_targets");
        goto_map.clear();
        let mut id_counter =
            lock_or_recover(&self.next_goto_target_id, "debug_adapter.next_goto_target_id");

        let mut targets = Vec::new();
        let search_start = (args.line - 5).max(1);
        let search_end = args.line + 5;

        if let Ok(validator) = AstBreakpointValidator::new(&content) {
            for line in search_start..=search_end {
                if self.cancel_requested.load(Ordering::Acquire) {
                    self.cancel_requested.store(false, Ordering::Release);
                    break;
                }
                if validator.is_executable_line(line) {
                    let id = *id_counter;
                    *id_counter += 1;
                    goto_map.insert(id, (source_path.clone(), line));
                    targets.push(GotoTarget {
                        id,
                        label: format!("Line {}", line),
                        line,
                        column: None,
                        end_line: None,
                        end_column: None,
                    });
                }
            }
        }
        drop(goto_map);
        drop(id_counter);

        let body = GotoTargetsResponseBody { targets };
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "gotoTargets".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    pub(super) fn handle_goto(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: GotoArguments = match arguments.and_then(|v| serde_json::from_value(v).ok()) {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "goto".to_string(),
                    body: None,
                    message: Some("Missing or invalid arguments".to_string()),
                };
            }
        };

        // Look up the goto target from our stored mapping
        let target_info = {
            let mut goto_map = lock_or_recover(&self.goto_targets, "debug_adapter.goto_targets");
            goto_map.remove(&args.target_id)
        };
        let (target_path, target_line) = match target_info {
            Some(info) => info,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "goto".to_string(),
                    body: None,
                    message: Some(format!("Unknown goto target id {}", args.target_id)),
                };
            }
        };

        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            // Set debugger file context for cross-file goto
            let file_cmd = format!("f {}\n", target_path);
            let _ = stdin.write_all(file_cmd.as_bytes());
            let _ = stdin.flush();
            let goto_cmd = format!("c {}\n", target_line);
            let _ = stdin.write_all(goto_cmd.as_bytes());
            let _ = stdin.flush();
            session.state = DebugState::Running;
            session.last_resume_mode = ResumeMode::Goto;
            session.variables.clear();
            session.stack_trace_cache = None;
            let t_id = session.thread_id;

            self.send_event(
                "continued",
                Some(json!({
                    "threadId": t_id,
                    "allThreadsContinued": true
                })),
            );

            DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "goto".to_string(),
                body: None,
                message: None,
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "goto".to_string(),
                body: None,
                message: Some("No active debug session".to_string()),
            }
        }
    }

    pub(super) fn handle_step_in_targets(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: StepInTargetsArguments =
            match arguments.and_then(|v| serde_json::from_value(v).ok()) {
                Some(a) => a,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "stepInTargets".to_string(),
                        body: None,
                        message: Some("Missing or invalid arguments".to_string()),
                    };
                }
            };

        let mut targets = Vec::new();

        // Extract the frame source path while session lock is held, then release.
        let frame_info = {
            let session_guard = lock_or_recover(&self.session, "debug_adapter.session");
            if let Some(ref session) = *session_guard {
                session
                    .stack_frames
                    .iter()
                    .find(|f| i64::from(f.id) == args.frame_id)
                    .map(|frame| (frame.source.path.clone(), frame.line))
            } else {
                None
            }
        };

        if let Some((source_path, frame_line)) = frame_info {
            // Defense-in-depth: validate even internal session paths
            if let Ok(validated_path) = self.validate_source_path(&source_path) {
                if let Ok(content) = std::fs::read_to_string(&validated_path) {
                    let line_idx = frame_line as usize;
                    if let Some(source_line) = content.lines().nth(line_idx.saturating_sub(1)) {
                        // Find function call patterns
                        if let Some(call_re) = STEP_IN_TARGET_CALL_RE.as_ref() {
                            for (idx, cap) in call_re.captures_iter(source_line).enumerate() {
                                if let Some(name) = cap.get(1) {
                                    targets.push(StepInTarget {
                                        id: idx as i64,
                                        label: name.as_str().to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let body = StepInTargetsResponseBody { targets };
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "stepInTargets".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    pub(super) fn handle_cancel(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        // cancel_requested field will be added by the integration task
        self.cancel_requested.store(true, Ordering::Release);
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "cancel".to_string(),
            body: None,
            message: None,
        }
    }

    pub(super) fn handle_restart_frame(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        DapMessage::Response {
            seq,
            request_seq,
            success: false,
            command: "restartFrame".to_string(),
            body: None,
            message: Some(
                "Perl does not support restarting execution from a specific stack frame"
                    .to_string(),
            ),
        }
    }

    pub(super) fn handle_terminate_threads(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        DapMessage::Response {
            seq,
            request_seq,
            success: false,
            command: "terminateThreads".to_string(),
            body: None,
            message: Some(
                "Perl threading model does not support targeted thread termination from the debugger"
                    .to_string(),
            ),
        }
    }
}
