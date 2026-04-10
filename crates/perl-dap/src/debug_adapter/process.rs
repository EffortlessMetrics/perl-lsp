//! Process lifecycle management: initialize, launch, attach, disconnect, terminate, restart.

use super::*;

impl DebugAdapter {
    /// Handle initialize request
    pub(super) fn handle_initialize(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        let supports_core = catalog_has_feature("dap.core");
        let supports_basic_breakpoints = catalog_has_feature("dap.breakpoints.basic");
        let supports_hit_conditions = catalog_has_feature("dap.breakpoints.hit_condition");
        let supports_log_points = catalog_has_feature("dap.breakpoints.logpoints");
        let supports_exceptions = catalog_has_feature("dap.exceptions.die");
        let supports_inline_values = catalog_has_feature("dap.inline_values");
        let supports_completions = catalog_has_feature("dap.completions");
        let supports_modules = catalog_has_feature("dap.modules");
        let supports_watchpoints = catalog_has_feature("dap.watchpoints");
        let supports_warn = catalog_has_feature("dap.exceptions.warn");
        let supports_any_exception = supports_exceptions || supports_warn;

        let mut filters = Vec::new();
        if supports_exceptions {
            filters.push(json!({
                "filter": "die",
                "label": "Perl die() and uncaught exceptions",
                "default": true
            }));
            filters.push(json!({
                "filter": "all",
                "label": "All Perl exception events",
                "default": false
            }));
        }
        if supports_warn {
            filters.push(json!({
                "filter": "warn",
                "label": "Perl warn() and Carp warnings",
                "default": false
            }));
        }
        let exception_breakpoint_filters = json!(filters);

        let capabilities = json!({
            "supportsConfigurationDoneRequest": supports_core,
            "supportsFunctionBreakpoints": supports_core,
            "supportsConditionalBreakpoints": supports_basic_breakpoints,
            "supportsHitConditionalBreakpoints": supports_hit_conditions,
            "supportsEvaluateForHovers": supports_core,
            "supportsStepBack": false,
            "supportsSetVariable": supports_core,
            "supportsRestartFrame": false,
            "supportsGotoTargetsRequest": supports_core,
            "supportsStepInTargetsRequest": false,
            "supportsCompletionsRequest": supports_completions,
            "supportsModulesRequest": supports_modules,
            "supportsRestartRequest": true,
            "supportsExceptionOptions": supports_any_exception,
            "supportsValueFormattingOptions": supports_core,
            "supportsExceptionInfoRequest": supports_any_exception,
            "supportTerminateDebuggee": supports_core,
            "supportsDelayedStackTraceLoading": false,
            "supportsLoadedSourcesRequest": true,
            "supportsLogPoints": supports_log_points,
            "supportsTerminateThreadsRequest": false,
            "supportsSetExpression": supports_core,
            "supportsTerminateRequest": supports_core,
            "supportsDataBreakpoints": supports_watchpoints,
            "supportsReadMemoryRequest": false,
            "supportsDisassembleRequest": false,
            "supportsCancelRequest": supports_core,
            "supportsBreakpointLocationsRequest": supports_basic_breakpoints,
            "supportsClipboardContext": false,
            "supportsSteppingGranularity": false,
            "supportsInstructionBreakpoints": false,
            "supportsExceptionFilterOptions": supports_any_exception,
            "supportsInlineValues": supports_inline_values,
            "exceptionBreakpointFilters": exception_breakpoint_filters
        });

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "initialize".to_string(),
            body: Some(capabilities),
            message: None,
        }
    }

    /// Handle launch request
    pub(super) fn handle_launch(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        if let Some(args) = arguments {
            // Store launch arguments for restart support
            *lock_or_recover(&self.last_launch_args, "debug_adapter.last_launch_args") =
                Some(args.clone());

            let program = args.get("program").and_then(|p| p.as_str()).unwrap_or("");

            // Set workspace root for path validation (prefer cwd, fall back to program's parent)
            let workspace = args
                .get("cwd")
                .and_then(|c| c.as_str())
                .map(PathBuf::from)
                .or_else(|| Path::new(program).parent().map(PathBuf::from));
            if let Some(ref root) = workspace {
                *lock_or_recover(&self.workspace_root, "debug_adapter.workspace_root") =
                    Some(root.clone());
            }

            let perl_args = args
                .get("args")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let stop_on_entry = args.get("stopOnEntry").and_then(|s| s.as_bool()).unwrap_or(false);

            let env_overrides = args
                .get("env")
                .and_then(Value::as_object)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|value| (key.clone(), value.to_string()))
                        })
                        .collect::<HashMap<String, String>>()
                })
                .unwrap_or_default();

            // Launch Perl debugger
            match self.launch_debugger(program, perl_args, stop_on_entry, env_overrides) {
                Ok(thread_id) => {
                    // Send stopped event if stop on entry
                    if stop_on_entry {
                        self.send_event(
                            "stopped",
                            Some(json!({
                                "reason": "entry",
                                "threadId": thread_id,
                                "allThreadsStopped": true
                            })),
                        );
                    }

                    DapMessage::Response {
                        seq,
                        request_seq,
                        success: true,
                        command: "launch".to_string(),
                        body: None,
                        message: None,
                    }
                }
                Err(e) => DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "launch".to_string(),
                    body: None,
                    message: Some(format!(
                        "Cannot start Perl debugger: {}. \
                         Check that 'perl' is on your PATH and that the file exists. \
                         You can set a custom interpreter path in your launch.json.",
                        e
                    )),
                },
            }
        } else {
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "launch".to_string(),
                body: None,
                message: Some(
                    "Debugger launch failed: no launch configuration was provided. \
                     Add a launch.json with a 'program' field pointing to your Perl script."
                        .to_string(),
                ),
            }
        }
    }

    /// Launch the Perl debugger
    pub(super) fn launch_debugger(
        &mut self,
        program: &str,
        args: Vec<String>,
        stop_on_entry: bool,
        env_overrides: HashMap<String, String>,
    ) -> Result<i32, String> {
        // Security: Validate program path before any process spawning
        // This prevents command injection via flag arguments (e.g., "-e malicious_code")
        // and ensures we're launching a real Perl script file.

        let program = program.trim();

        // Reject empty or whitespace-only paths
        if program.is_empty() {
            return Err(
                "No Perl script was specified. Set the 'program' field in your launch.json \
                 to the path of the script you want to debug."
                    .to_string(),
            );
        }

        // Validate that the program is a regular file (not a directory, device, etc.)
        // Using metadata().is_file() is more robust than exists() because:
        // - exists() returns true for directories
        // - exists() returns true for symlinks to non-files
        // - is_file() specifically checks for regular files
        let path = Path::new(program);
        match std::fs::metadata(path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(format!(
                        "'{}' is not a file. Update the 'program' field in your launch.json \
                         to point to a Perl script (.pl or .t).",
                        program
                    ));
                }
            }
            Err(e) => {
                return Err(format!(
                    "Cannot find '{}': {}. \
                     Check that the 'program' path in your launch.json is correct.",
                    program, e
                ));
            }
        }

        // Enforce workspace-bound launch paths when a workspace root is known.
        // This prevents launching scripts outside the active project tree.
        let workspace_root =
            lock_or_recover(&self.workspace_root, "debug_adapter.workspace_root").clone();
        if let Some(root) = workspace_root.as_ref() {
            security::validate_path(path, root).map_err(|e| {
                format!(
                    "The script '{}' is outside your workspace folder. \
                     Only scripts within the open workspace can be debugged. \
                     Details: {}",
                    program, e
                )
            })?;
        }

        let mut cmd = Command::new("perl");
        cmd.arg("-d");

        // Perl debugger stops on the first line by default
        let _ = stop_on_entry; // currently unused

        // Use -- to separate flags from script name, preventing argument injection
        // if program starts with -
        cmd.arg("--");
        cmd.arg(program);
        cmd.args(&args);
        cmd.envs(env_overrides);

        // Set up pipes
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(child) => {
                let thread_id = {
                    if let Ok(mut counter) = self.thread_counter.lock() {
                        *counter += 1;
                        *counter
                    } else {
                        tracing::warn!("Failed to lock thread counter, using 1");
                        1
                    }
                };

                let session = DebugSession {
                    process: child,
                    state: DebugState::Running,
                    stack_frames: Vec::new(),
                    variables: HashMap::new(),
                    thread_id,
                    last_resume_mode: ResumeMode::Unknown,
                };

                if let Ok(mut guard) = self.session.lock() {
                    *guard = Some(session);
                } else {
                    return Err(
                        "Debugger could not be started: an internal state error occurred. \
                         Try stopping the debug session and relaunching."
                            .to_string(),
                    );
                }

                // Apply any function breakpoints configured before launch.
                self.apply_stored_function_breakpoints();

                // Start output reader thread
                self.start_output_reader();

                Ok(thread_id)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Start thread to read debugger output with enhanced error recovery
    pub(super) fn start_output_reader(&self) {
        let session = self.session.clone();
        let seq = self.seq.clone();
        let sender = self.event_sender.clone();
        let recent_output = self.recent_output.clone();
        let breakpoints = self.breakpoints.clone();
        let exception_break_on_die = self.exception_break_on_die.clone();
        let exception_break_on_warn = self.exception_break_on_warn.clone();
        let last_exception_message = self.last_exception_message.clone();
        let tcp_session = self.tcp_session.clone();
        let attached_pid = self.attached_pid.clone();

        thread::spawn(move || {
            // Perl's debugger prompt and evaluation output are emitted on stderr.
            // Prefer stderr as the control stream, with stdout as a fallback.
            let control_stream: Option<Box<dyn Read + Send>> = {
                if let Ok(mut guard) = session.lock() {
                    guard.as_mut().and_then(|s| {
                        if let Some(stderr) = s.process.stderr.take() {
                            Some(Box::new(stderr) as Box<dyn Read + Send>)
                        } else {
                            s.process
                                .stdout
                                .take()
                                .map(|stdout| Box::new(stdout) as Box<dyn Read + Send>)
                        }
                    })
                } else {
                    tracing::warn!("Failed to lock session in output reader");
                    None
                }
            };

            let Some(control_stream) = control_stream else {
                tracing::warn!(
                    "No debugger output stream available - output reader thread exiting"
                );
                // Send termination event
                if let Some(ref sender) = sender {
                    emit_event_safe(
                        sender,
                        &seq,
                        "terminated",
                        Some(json!({"reason": "no_debugger_stream"})),
                    );
                }
                DebugAdapter::clear_active_session_state_with_state(
                    &session,
                    &tcp_session,
                    &attached_pid,
                );
                return;
            };

            let mut reader = BufReader::new(control_stream);
            let mut line = String::new();

            let mut current_file = String::new();
            let mut current_func = String::new();
            let mut current_line = 0;
            let mut _debugger_ready = false;

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        tracing::debug!("Perl debugger process terminated");
                        DebugAdapter::clear_active_session_state_with_state(
                            &session,
                            &tcp_session,
                            &attached_pid,
                        );
                        break;
                    }
                    Ok(_) => {
                        let text = line.trim_end().to_string();
                        let sanitized_text = if let Some(re) = ansi_escape_re() {
                            re.replace_all(&text, "").into_owned()
                        } else {
                            text.clone()
                        };
                        let normalized_text = DebugAdapter::normalize_debugger_output_line(&text);
                        let analysis_text = if normalized_text.is_empty() {
                            sanitized_text.trim().to_string()
                        } else {
                            normalized_text
                        };
                        tracing::trace!(output = %text, "Debugger output");
                        {
                            let mut output = lock_or_recover(
                                &recent_output,
                                "debug_adapter.recent_output_reader",
                            );
                            if output.len() >= RECENT_OUTPUT_MAX_LINES {
                                let _ = output.pop_front();
                            }
                            output.push_back(text.clone());
                        }

                        // Send all output to client with error handling
                        if let Some(ref sender) = sender
                            && !emit_event_safe(
                                sender,
                                &seq,
                                "output",
                                Some(json!({
                                    "category": "stdout",
                                    "output": format!("{}\n", text)
                                })),
                            )
                        {
                            tracing::warn!(
                                "Failed to send output event - client may have disconnected"
                            );
                            break; // Exit the loop if client is gone
                        }

                        // Enhanced context information parsing with multiple patterns
                        let mut context_updated = false;

                        // Try main context pattern
                        if let Some(re) = context_re()
                            && let Some(caps) = re.captures(&analysis_text)
                        {
                            if let Some(func) = caps.name("func") {
                                current_func = func.as_str().to_string();
                                context_updated = true;
                            }
                            if let Some(file) = caps.name("file").or_else(|| caps.name("file2")) {
                                current_file = file.as_str().to_string();
                                context_updated = true;
                            }
                            if let Some(line_num) = caps.name("line").or_else(|| caps.name("line2"))
                            {
                                current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                                context_updated = true;
                            }
                        }

                        // Try stack frame pattern as fallback
                        if !context_updated
                            && let Some(re) = stack_frame_re()
                            && let Some(caps) = re.captures(&analysis_text)
                        {
                            if let Some(func) = caps.name("func") {
                                current_func = func.as_str().to_string();
                            }
                            if let Some(file) = caps.name("file") {
                                current_file = file.as_str().to_string();
                            }
                            if let Some(line_num) = caps.name("line") {
                                current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                            }
                            context_updated = true;
                        }

                        // Check for errors that might provide location info
                        if !context_updated
                            && let Some(re) = error_re()
                            && let Some(caps) = re.captures(&analysis_text)
                        {
                            if let Some(file) = caps.name("file") {
                                current_file = file.as_str().to_string();
                            }
                            if let Some(line_num) = caps.name("line") {
                                current_line = line_num.as_str().parse::<i32>().unwrap_or(0);
                            }
                            context_updated = true;

                            // Send error event to client
                            if let Some(ref sender) = sender {
                                emit_event_safe(
                                    sender,
                                    &seq,
                                    "output",
                                    Some(json!({
                                        "category": "stderr",
                                        "output": format!("Error: {}\n", text)
                                    })),
                                );
                            }
                        }

                        if context_updated {
                            let break_on_die =
                                exception_break_on_die.lock().map(|guard| *guard).unwrap_or(false);
                            let break_on_warn =
                                exception_break_on_warn.lock().map(|guard| *guard).unwrap_or(false);
                            let is_exception_line =
                                exception_re().is_some_and(|re| re.is_match(&analysis_text));
                            let is_warning_line =
                                warning_re().is_some_and(|re| re.is_match(&analysis_text));
                            let exception_match = break_on_die && is_exception_line;
                            let warning_match =
                                break_on_warn && is_warning_line && !is_exception_line;

                            // Store exception message for exceptionInfo request
                            if exception_match || warning_match {
                                if let Ok(mut guard) = last_exception_message.lock() {
                                    *guard = Some(analysis_text.clone());
                                }
                            }

                            let mut should_emit_stopped = false;
                            let mut should_auto_continue = false;
                            let mut stop_reason = "step".to_string();
                            let mut logpoint_messages: Vec<String> = Vec::new();

                            let thread_id = {
                                let Ok(mut guard) = session.lock() else {
                                    tracing::warn!(
                                        "Failed to lock session when processing debugger context"
                                    );
                                    continue;
                                };

                                if let Some(ref mut s) = *guard {
                                    if !current_file.is_empty() && current_line > 0 {
                                        s.stack_frames = vec![StackFrame {
                                            id: 1,
                                            name: if current_func.is_empty() {
                                                "main".to_string()
                                            } else {
                                                current_func.clone()
                                            },
                                            source: Source {
                                                name: Some(
                                                    std::path::Path::new(&current_file)
                                                        .file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or(&current_file)
                                                        .to_string(),
                                                ),
                                                path: current_file.clone(),
                                                source_reference: None,
                                            },
                                            line: current_line,
                                            column: 1,
                                            end_line: None,
                                            end_column: None,
                                        }];
                                    }

                                    if matches!(s.state, DebugState::Running) {
                                        should_emit_stopped = true;
                                        let resume_mode = s.last_resume_mode.clone();

                                        let breakpoint_outcome =
                                            if matches!(resume_mode, ResumeMode::Continue)
                                                && !current_file.is_empty()
                                                && current_line > 0
                                            {
                                                breakpoints.register_breakpoint_hit(
                                                    &current_file,
                                                    i64::from(current_line),
                                                )
                                            } else {
                                                BreakpointHitOutcome::default()
                                            };

                                        if exception_match || warning_match {
                                            stop_reason = "exception".to_string();
                                            s.state = DebugState::Stopped;
                                        } else if breakpoint_outcome.matched {
                                            logpoint_messages = breakpoint_outcome.log_messages;
                                            if breakpoint_outcome.should_stop {
                                                stop_reason = "breakpoint".to_string();
                                                s.state = DebugState::Stopped;
                                            } else {
                                                if let Some(stdin) = s.process.stdin.as_mut() {
                                                    let _ = stdin.write_all(b"c\n");
                                                    let _ = stdin.flush();
                                                }
                                                s.state = DebugState::Running;
                                                s.last_resume_mode = ResumeMode::Continue;
                                                should_auto_continue = true;
                                            }
                                        } else {
                                            s.state = DebugState::Stopped;
                                        }

                                        if !should_auto_continue {
                                            s.last_resume_mode = ResumeMode::Unknown;
                                        }
                                    }

                                    s.thread_id
                                } else {
                                    continue;
                                }
                            };

                            if let Some(ref sender) = sender {
                                for message in logpoint_messages {
                                    emit_event_safe(
                                        sender,
                                        &seq,
                                        "output",
                                        Some(json!({
                                            "category": "console",
                                            "output": format!("{message}\n")
                                        })),
                                    );
                                }
                            }

                            if should_auto_continue {
                                continue;
                            }

                            if should_emit_stopped
                                && let Some(ref sender) = sender
                                && !emit_event_safe(
                                    sender,
                                    &seq,
                                    "stopped",
                                    Some(json!({
                                        "reason": stop_reason,
                                        "threadId": thread_id,
                                        "allThreadsStopped": true
                                    })),
                                )
                            {
                                tracing::warn!(
                                    "Failed to send stopped event - client disconnected"
                                );
                                return;
                            }
                            continue;
                        }

                        // Detect debugger prompt (stopped state) with enhanced pattern matching
                        if prompt_re().is_some_and(|re| re.is_match(&sanitized_text)) {
                            _debugger_ready = true;
                            let thread_id = {
                                let Ok(mut guard) = session.lock() else {
                                    tracing::warn!(
                                        "Failed to lock session when processing debugger prompt"
                                    );
                                    continue;
                                };
                                if let Some(ref mut s) = *guard {
                                    // Create stack frame with enhanced context validation
                                    if !current_file.is_empty() && current_line > 0 {
                                        let frame = StackFrame {
                                            id: 1,
                                            name: if current_func.is_empty() {
                                                "main".to_string()
                                            } else {
                                                current_func.clone()
                                            },
                                            source: Source {
                                                name: Some(
                                                    std::path::Path::new(&current_file)
                                                        .file_name()
                                                        .and_then(|n| n.to_str())
                                                        .unwrap_or(&current_file)
                                                        .to_string(),
                                                ),
                                                path: current_file.clone(),
                                                source_reference: None,
                                            },
                                            line: current_line,
                                            column: 1,
                                            end_line: None,
                                            end_column: None,
                                        };
                                        s.stack_frames = vec![frame];
                                    } else {
                                        // Provide a fallback frame for when we don't have perfect context
                                        let frame = StackFrame {
                                            id: 1,
                                            name: "main".to_string(),
                                            source: Source {
                                                name: Some("<unknown>".to_string()),
                                                path: "<unknown>".to_string(),
                                                source_reference: None,
                                            },
                                            line: 1,
                                            column: 1,
                                            end_line: None,
                                            end_column: None,
                                        };
                                        s.stack_frames = vec![frame];
                                    }
                                    s.state = DebugState::Stopped;
                                    s.thread_id
                                } else {
                                    continue;
                                }
                            };

                            // Send stopped event with robust error handling
                            if let Some(ref sender) = sender
                                && !emit_event_safe(
                                    sender,
                                    &seq,
                                    "stopped",
                                    Some(json!({
                                        "reason": "step",
                                        "threadId": thread_id,
                                        "allThreadsStopped": true
                                    })),
                                )
                            {
                                tracing::warn!(
                                    "Failed to send stopped event - client disconnected"
                                );
                                return; // Exit thread
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Error reading from debugger");
                        // Send termination event before exiting
                        if let Some(ref sender) = sender {
                            emit_event_safe(
                                sender,
                                &seq,
                                "terminated",
                                Some(json!({"reason": "read_error", "error": e.to_string()})),
                            );
                        }
                        DebugAdapter::clear_active_session_state_with_state(
                            &session,
                            &tcp_session,
                            &attached_pid,
                        );
                        break;
                    }
                }
            }
        });
    }

    /// Handle attach request
    ///
    /// Attaches to a running Perl process. Supports two modes:
    /// 1. TCP attachment - Connect to Perl::LanguageServer DAP via host:port
    /// 2. Process ID attachment - Signal-control mode for local Perl process
    ///
    /// For TCP attachment, the arguments should contain:
    /// - `host`: Hostname or IP address (default: "localhost")
    /// - `port`: Port number (default: 13603)
    /// - `timeout`: Connection timeout in milliseconds (optional)
    ///
    /// # Current Implementation
    ///
    /// TCP attachment is implemented with socket support.
    /// Process ID attachment is implemented in signal-control mode (pause/continue
    /// signaling and thread identity), with limited stack/evaluate capabilities
    /// unless a debugger transport is active.
    pub(super) fn handle_attach(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        // Parse attach arguments
        if let Some(args) = arguments {
            let process_id = args.get("processId").and_then(|p| p.as_u64()).map(|p| p as u32);

            // PID attachment mode: best-effort process control without requiring TCP shim transport.
            if let Some(pid) = process_id {
                if pid == 0 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some("processId must be greater than zero".to_string()),
                    };
                }

                // Reset existing process/tcp attachment state before switching to PID mode.
                self.clear_active_session_state();

                if let Ok(mut guard) = self.attached_pid.lock() {
                    *guard = Some(pid);
                }

                let stop_on_entry =
                    args.get("stopOnEntry").and_then(|s| s.as_bool()).unwrap_or(false);
                let thread_id = Self::i64_to_i32_saturating(i64::from(pid));

                // Always emit the "attach" stopped event to signal the client that the
                // debugger is connected and paused.
                self.send_event(
                    "stopped",
                    Some(json!({
                        "reason": "attach",
                        "threadId": thread_id,
                        "allThreadsStopped": true
                    })),
                );

                // When stopOnEntry is requested, emit an additional "entry" stopped event
                // so the IDE pauses at the first available program location.
                if stop_on_entry {
                    self.send_event(
                        "stopped",
                        Some(json!({
                            "reason": "entry",
                            "threadId": thread_id,
                            "allThreadsStopped": true,
                            "description": "Paused on entry"
                        })),
                    );
                }

                tracing::info!(
                    pid,
                    stop_on_entry,
                    "Attach request: Process ID attachment (signal-control mode)"
                );

                DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "attach".to_string(),
                    body: Some(json!({
                        "threadId": thread_id,
                        "processId": pid,
                        "mode": "processId"
                    })),
                    message: Some(
                        "Attached in signal-control mode. Stack/evaluate are limited without a \
                         debugger transport."
                            .to_string(),
                    ),
                }
            } else {
                // Extract host and port for TCP attachment.
                let host = args.get("host").and_then(|h| h.as_str()).unwrap_or("localhost");
                let raw_port = args.get("port").and_then(|p| p.as_u64()).unwrap_or(13603);
                if raw_port > 65535 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(format!("Port {raw_port} out of range (must be 1-65535)")),
                    };
                }
                let port = raw_port as u16;
                let timeout = args.get("timeout").and_then(|t| t.as_u64()).map(|t| t as u32);
                let stop_on_entry =
                    args.get("stopOnEntry").and_then(|s| s.as_bool()).unwrap_or(false);

                // Validate arguments.
                if host.trim().is_empty() {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some("Host cannot be empty".to_string()),
                    };
                }

                if port == 0 {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some("Port must be in range 1-65535".to_string()),
                    };
                }

                if let Some(t) = timeout {
                    if t == 0 {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "attach".to_string(),
                            body: None,
                            message: Some(
                                "Timeout must be greater than 0 milliseconds".to_string(),
                            ),
                        };
                    }
                    if t > 300_000 {
                        return DapMessage::Response {
                            seq,
                            request_seq,
                            success: false,
                            command: "attach".to_string(),
                            body: None,
                            message: Some(
                                "Timeout cannot exceed 300000 milliseconds (5 minutes)".to_string(),
                            ),
                        };
                    }
                }

                // TCP attachment mode (IMPLEMENTED)
                let mut config = TcpAttachConfig::new(host.to_string(), port);
                if let Some(t) = timeout {
                    config = config.with_timeout(t);
                }

                // Validate configuration
                if let Err(e) = config.validate() {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(format!("Invalid attach configuration: {}", e)),
                    };
                }

                // Create TCP attach session
                let mut session = TcpAttachSession::new();

                // Set up event channel for TCP events
                let (tx, rx) = channel::<DapEvent>();
                session.set_event_sender(tx);

                // Attempt to connect
                match session.connect(&config) {
                    Ok(()) => {
                        // Store session
                        if let Ok(mut guard) = self.tcp_session.lock() {
                            *guard = Some(session);
                        }

                        // Start reader thread
                        if let Ok(mut guard) = self.tcp_session.lock() {
                            if let Some(ref mut s) = *guard {
                                if let Err(e) = s.start_reader() {
                                    tracing::error!(error = %e, "Failed to start TCP reader");
                                    return DapMessage::Response {
                                        seq,
                                        request_seq,
                                        success: false,
                                        command: "attach".to_string(),
                                        body: None,
                                        message: Some(format!("Failed to start TCP reader: {}", e)),
                                    };
                                }
                            }
                        }

                        // Start event handler thread for TCP events
                        let seq_counter = self.seq.clone();
                        let event_sender = self.event_sender.clone();
                        thread::spawn(move || {
                            while let Ok(event) = rx.recv() {
                                match event {
                                    DapEvent::Output { category, output } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "output".to_string(),
                                                body: Some(json!({
                                                    "category": category,
                                                    "output": output
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Stopped { reason, thread_id } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "stopped".to_string(),
                                                body: Some(json!({
                                                    "reason": reason,
                                                    "threadId": thread_id,
                                                    "allThreadsStopped": true
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Continued { thread_id } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "continued".to_string(),
                                                body: Some(json!({
                                                    "threadId": thread_id,
                                                    "allThreadsContinued": true
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Terminated { reason } => {
                                        if let Some(ref sender) = event_sender {
                                            let mut seq_lock = seq_counter
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner());
                                            *seq_lock += 1;
                                            let _ = sender.send(DapMessage::Event {
                                                seq: *seq_lock,
                                                event: "terminated".to_string(),
                                                body: Some(json!({
                                                    "reason": reason
                                                })),
                                            });
                                        }
                                    }
                                    DapEvent::Error { message } => {
                                        tracing::error!(message, "TCP attach error");
                                    }
                                }
                            }
                        });

                        // When stopOnEntry is requested, emit a stopped event so the IDE
                        // pauses at the first available program location after the TCP
                        // attach handshake completes.
                        if stop_on_entry {
                            self.send_event(
                                "stopped",
                                Some(json!({
                                    "reason": "entry",
                                    "threadId": 1,
                                    "allThreadsStopped": true,
                                    "description": "Paused on entry"
                                })),
                            );
                        }

                        tracing::info!(host, port, stop_on_entry, "TCP attach successful");

                        DapMessage::Response {
                            seq,
                            request_seq,
                            success: true,
                            command: "attach".to_string(),
                            body: None,
                            message: None,
                        }
                    }
                    Err(e) => DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "attach".to_string(),
                        body: None,
                        message: Some(format!(
                            "Cannot attach to Perl debugger at {}:{} ({}ms timeout): {}. \
                             Make sure the Perl process was started with \
                             'PERLDB_OPTS=\"RemotePort={}:{}\"' \
                             and is still running before attaching.",
                            config.host,
                            config.port,
                            config.timeout_ms.unwrap_or(30000),
                            e,
                            config.host,
                            config.port,
                        )),
                    },
                }
            }
        } else {
            // No arguments provided
            DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "attach".to_string(),
                body: None,
                message: Some(
                    "Missing attach arguments. Provide either 'processId' for process attachment \
                     or 'host' and 'port' for TCP attachment."
                        .to_string(),
                ),
            }
        }
    }

    /// Clear active process session, TCP session, and PID-attach mode state.
    pub(super) fn clear_active_session_state(&self) {
        Self::clear_active_session_state_with_state(
            &self.session,
            &self.tcp_session,
            &self.attached_pid,
        );
    }

    pub(super) fn clear_active_session_state_with_state(
        session: &Arc<Mutex<Option<DebugSession>>>,
        tcp_session: &Arc<Mutex<Option<TcpAttachSession>>>,
        attached_pid: &Arc<Mutex<Option<u32>>>,
    ) {
        // Terminate the debug session
        if let Ok(mut guard) = session.lock()
            && let Some(mut active_session) = guard.take()
        {
            if !Self::terminate_child_process(&mut active_session.process) {
                tracing::warn!("Failed to ensure debug session process termination");
            }
            active_session.state = DebugState::Terminated;
        }

        // Disconnect TCP session if active
        if let Ok(mut guard) = tcp_session.lock()
            && let Some(ref mut tcp_session) = *guard
        {
            let _ = tcp_session.disconnect();
        }
        if let Ok(mut guard) = tcp_session.lock() {
            *guard = None;
        }

        // Clear PID attach mode.
        if let Ok(mut guard) = attached_pid.lock() {
            *guard = None;
        }
    }

    pub(super) fn wait_for_child_exit(process: &mut Child, timeout: Duration) -> bool {
        if let Ok(Some(_)) = process.try_wait() {
            return true;
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            match process.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to poll debug session process");
                    return false;
                }
            }
        }

        false
    }

    pub(super) fn terminate_child_process(process: &mut Child) -> bool {
        if Self::wait_for_child_exit(process, Duration::from_millis(0)) {
            return true;
        }

        #[cfg(unix)]
        {
            let pid = process.id();
            match signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                Ok(()) => {
                    if Self::wait_for_child_exit(
                        process,
                        Duration::from_millis(DEBUG_SESSION_TERMINATE_WAIT_MS),
                    ) {
                        return true;
                    }
                }
                Err(e) => {
                    tracing::warn!(pid, error = %e, "Failed to send SIGTERM to process");
                }
            }
        }

        if let Err(e) = process.kill() {
            tracing::warn!(error = %e, "Failed to terminate process");
        }
        Self::wait_for_child_exit(process, Duration::from_millis(DEBUG_SESSION_TERMINATE_WAIT_MS))
    }

    /// Handle disconnect request
    pub(super) fn handle_disconnect(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<DisconnectArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        self.clear_active_session_state();

        // Send terminated event
        self.send_event("terminated", None);

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "disconnect".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle terminate request
    pub(super) fn handle_terminate(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: Option<TerminateArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        let restart = args.and_then(|a| a.restart);

        self.clear_active_session_state();

        let terminated_body = restart.map(|restart| json!({ "restart": restart }));
        self.send_event("terminated", terminated_body);

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "terminate".to_string(),
            body: None,
            message: None,
        }
    }

    /// Apply stored function breakpoints to the active debugger session.
    pub(super) fn apply_stored_function_breakpoints(&self) {
        let names =
            self.function_breakpoints.lock().map(|stored| stored.clone()).unwrap_or_default();
        if names.is_empty() {
            return;
        }

        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            for name in names {
                let cmd = format!("b {name}\n");
                let _ = stdin.write_all(cmd.as_bytes());
            }
            let _ = stdin.flush();
        }
    }

    /// Handle configurationDone request
    pub(super) fn handle_configuration_done(&self, seq: i64, request_seq: i64) -> DapMessage {
        // Send initial command to get the debugger started
        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            && let Some(stdin) = session.process.stdin.as_mut()
        {
            // Send initial 'l' command to list current location
            let _ = stdin.write_all(b"l\n");
            let _ = stdin.flush();
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "configurationDone".to_string(),
            body: None,
            message: None,
        }
    }

    /// Handle threads request
    pub(super) fn handle_threads(&self, seq: i64, request_seq: i64) -> DapMessage {
        let threads = if let Some(ref session) =
            *lock_or_recover(&self.session, "debug_adapter.session")
        {
            vec![json!({
                "id": session.thread_id,
                "name": "Main Thread"
            })]
        } else if let Some(pid) = *lock_or_recover(&self.attached_pid, "debug_adapter.attached_pid")
        {
            vec![json!({
                "id": Self::i64_to_i32_saturating(i64::from(pid)),
                "name": format!("Attached Process ({pid})")
            })]
        } else if lock_or_recover(&self.tcp_session, "debug_adapter.tcp_session").is_some() {
            vec![json!({ "id": 1, "name": "TCP Attached Thread" })]
        } else {
            vec![]
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "threads".to_string(),
            body: Some(json!({
                "threads": threads
            })),
            message: None,
        }
    }

    /// Send continue/resume signal to process.
    ///
    /// On Unix, sends SIGCONT. On Windows there is no direct equivalent of SIGCONT
    /// for externally-attached processes; the function returns `false` and logs a
    /// structured warning. Note that `handle_continue` emits the DAP `continued`
    /// event unconditionally, so the client will not be left in a stuck state even
    /// when this returns `false`.
    pub(super) fn send_continue_signal(&self, pid: u32) -> bool {
        if pid == 0 {
            tracing::warn!("send_continue_signal called with pid 0, ignoring");
            return false;
        }
        #[cfg(unix)]
        {
            let pid_i = pid as i32;
            match signal::kill(Pid::from_raw(pid_i), Signal::SIGCONT) {
                Ok(()) => {
                    tracing::info!("Sent SIGCONT to process {}", pid);
                    true
                }
                Err(e) => {
                    tracing::warn!("Failed to send SIGCONT to process {}: {}", pid, e);
                    false
                }
            }
        }
        #[cfg(windows)]
        {
            // Windows has no direct SIGCONT equivalent for external processes.
            // The caller (handle_continue) emits the continued event regardless.
            tracing::warn!(
                "send_continue_signal: SIGCONT not available on Windows for pid {}",
                pid
            );
            false
        }
        #[cfg(not(any(unix, windows)))]
        {
            tracing::warn!("send_continue_signal: unsupported platform for pid {}", pid);
            false
        }
    }

    /// Send interrupt signal to process (cross-platform).
    ///
    /// On Unix, sends SIGINT. On Windows, tries `GenerateConsoleCtrlEvent` first
    /// (works when the target is in the same console group), then falls back to
    /// writing the interrupt character to debugger stdin (session mode only).
    /// Returns `false` on unsupported platforms.
    pub(super) fn send_interrupt_signal(&self, pid: u32) -> bool {
        if pid == 0 {
            tracing::warn!("send_interrupt_signal called with pid 0, ignoring");
            return false;
        }
        #[cfg(unix)]
        {
            let pid_i = pid as i32;
            match signal::kill(Pid::from_raw(pid_i), Signal::SIGINT) {
                Ok(()) => {
                    tracing::info!("Sent SIGINT to process {}", pid);
                    true
                }
                Err(e) => {
                    tracing::warn!("Failed to send SIGINT to process {}: {}", pid, e);
                    false
                }
            }
        }
        #[cfg(windows)]
        {
            use winapi::um::wincon::{CTRL_C_EVENT, GenerateConsoleCtrlEvent};
            // Try GenerateConsoleCtrlEvent first (works for processes in same console group).
            // SAFETY: GenerateConsoleCtrlEvent is a stable Win32 API that takes two POD-by-value
            // arguments (a u32 control code and a u32 process group id) and returns a BOOL. It
            // has no preconditions on the calling thread or process state, holds no caller-owned
            // resources, and cannot dereference invalid memory because both arguments are passed
            // by value. The only failure mode is the call returning 0 (FALSE), which we handle
            // explicitly via the `if result != 0` check below.
            let result = unsafe { GenerateConsoleCtrlEvent(CTRL_C_EVENT, pid) };
            if result != 0 {
                tracing::info!("Sent Ctrl+C event to process {}", pid);
                return true;
            }
            tracing::warn!(
                "GenerateConsoleCtrlEvent failed for pid {}, trying stdin fallback",
                pid
            );

            // Fallback: write interrupt character to debugger stdin (session mode only).
            if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
            {
                if let Some(stdin) = session.process.stdin.as_mut() {
                    match stdin.write_all(b"\x03\n") {
                        Ok(()) => {
                            let _ = stdin.flush();
                            tracing::info!("Sent interrupt via stdin to process {}", pid);
                            true
                        }
                        Err(e) => {
                            tracing::error!("Failed to send interrupt to process {}: {}", pid, e);
                            if Self::terminate_child_process(&mut session.process) {
                                tracing::warn!("Terminated process {} as fallback", pid);
                                session.state = DebugState::Terminated;
                                true
                            } else {
                                tracing::error!("Failed to terminate process {}", pid);
                                false
                            }
                        }
                    }
                } else {
                    tracing::warn!("No stdin handle for process {}", pid);
                    false
                }
            } else {
                // attached_pid mode: GenerateConsoleCtrlEvent was already attempted above.
                tracing::warn!(
                    "GenerateConsoleCtrlEvent failed and no session active for pid {}",
                    pid
                );
                false
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            tracing::warn!("send_interrupt_signal: unsupported platform for pid {}", pid);
            false
        }
    }

    /// Handle restart request
    ///
    /// Restarts the debug session by tearing down the current session and
    /// re-launching with stored (or updated) launch arguments. If no previous
    /// launch configuration is available, returns an error.
    pub(super) fn handle_restart(
        &mut self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: Option<RestartArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());

        // Determine launch args: prefer restart-provided args, then stored args
        let updated_args = args.and_then(|a| a.arguments);

        let launch_args = if let Some(new_args) = updated_args {
            new_args
        } else {
            let stored = lock_or_recover(&self.last_launch_args, "debug_adapter.last_launch_args");
            match stored.clone() {
                Some(args) => args,
                None => {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: false,
                        command: "restart".to_string(),
                        body: None,
                        message: Some(
                            "Cannot restart: no previous launch configuration found. \
                             Start a debug session first, then use Restart."
                                .to_string(),
                        ),
                    };
                }
            }
        };

        self.clear_active_session_state();
        self.handle_launch(seq, request_seq, Some(launch_args))
    }
}
