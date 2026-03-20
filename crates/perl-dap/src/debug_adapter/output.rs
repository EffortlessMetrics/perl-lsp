//! Output handling: source retrieval, loaded sources, modules, inline values, exception info.

use super::*;

impl DebugAdapter {
    /// Handle inlineValues request (custom)
    ///
    /// Queries the Perl debugger for runtime variable values and returns
    /// inline value hints with Perl-idiomatic formatting.
    pub(super) fn handle_inline_values(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let Some(args) = arguments else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some("Missing arguments".to_string()),
            };
        };

        let args: InlineValuesArguments = match serde_json::from_value(args) {
            Ok(parsed) => parsed,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "inlineValues".to_string(),
                    body: None,
                    message: Some(format!("Invalid arguments: {}", e)),
                };
            }
        };

        let Some(source_path) = args.source.path else {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some("inlineValues requires source.path".to_string()),
            };
        };

        if args.start_line <= 0 || args.end_line <= 0 {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some("inlineValues requires positive startLine/endLine".to_string()),
            };
        }

        let start_line = args.start_line.min(args.end_line);
        let end_line = args.end_line.max(args.start_line);
        let content = match std::fs::read_to_string(&source_path) {
            Ok(content) => content,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "inlineValues".to_string(),
                    body: None,
                    message: Some(format!("Failed to read source file: {}", e)),
                };
            }
        };

        // Query runtime variable values from the debugger
        let runtime_values = self.query_inline_variable_values(&content, start_line, end_line);

        let inline_values = collect_inline_values_with_runtime(
            &content,
            start_line,
            end_line,
            runtime_values.as_ref(),
        );
        let body = InlineValuesResponseBody { inline_values };

        match serde_json::to_value(&body) {
            Ok(body) => DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "inlineValues".to_string(),
                body: Some(body),
                message: None,
            },
            Err(e) => DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "inlineValues".to_string(),
                body: None,
                message: Some(format!("Failed to serialize inlineValues response: {}", e)),
            },
        }
    }

    /// Query runtime variable values for inline display.
    ///
    /// Extracts variable names from source, then queries the Perl debugger
    /// for each variable's current value.
    fn query_inline_variable_values(
        &self,
        source: &str,
        start_line: i64,
        end_line: i64,
    ) -> Option<HashMap<String, String>> {
        let var_names = extract_variable_names(source, start_line, end_line);
        if var_names.is_empty() {
            return None;
        }

        // Check for active debug session
        let has_session = lock_or_recover(&self.session, "debug_adapter.session").is_some();
        if !has_session {
            return None;
        }

        let mut values = HashMap::new();

        for var_name in &var_names {
            if self.cancel_requested.load(Ordering::Acquire) {
                self.cancel_requested.store(false, Ordering::Release);
                return Some(values);
            }

            let sigil = var_name.chars().next().unwrap_or('$');
            let cmd = match sigil {
                '@' => format!("p scalar {}", var_name),
                '%' => format!("p scalar(keys {})", var_name),
                _ => format!("p {}", var_name),
            };

            let output_frame_markers = {
                let mut session_guard = lock_or_recover(&self.session, "debug_adapter.session");
                if let Some(ref mut session) = *session_guard {
                    if let Some(stdin) = session.process.stdin.as_mut() {
                        let commands = vec![cmd];
                        self.send_framed_debugger_commands(stdin, &commands).ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let result = output_frame_markers.and_then(|(begin, end)| {
                self.capture_framed_debugger_output(&begin, &end, DEBUGGER_QUERY_WAIT_MS)
            });

            if let Some(lines) = result {
                let raw: String = lines.join(" ").trim().to_string();
                if !raw.is_empty() {
                    values.insert(var_name.clone(), raw);
                }
            }
        }

        if values.is_empty() { None } else { Some(values) }
    }

    pub(super) fn handle_source(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: SourceArguments = match arguments.and_then(|v| serde_json::from_value(v).ok()) {
            Some(a) => a,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "source".to_string(),
                    body: None,
                    message: Some("Missing or invalid arguments".to_string()),
                };
            }
        };

        let path = match args.source.and_then(|s| s.path) {
            Some(p) => p,
            None => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "source".to_string(),
                    body: None,
                    message: Some("source.path is required".to_string()),
                };
            }
        };

        // Validate path against workspace root to prevent path traversal
        let validated_path = match self.validate_source_path(&path) {
            Ok(p) => p,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "source".to_string(),
                    body: None,
                    message: Some(e),
                };
            }
        };

        match std::fs::read_to_string(&validated_path) {
            Ok(content) => {
                let body =
                    SourceResponseBody { content, mime_type: Some("text/x-perl".to_string()) };
                DapMessage::Response {
                    seq,
                    request_seq,
                    success: true,
                    command: "source".to_string(),
                    body: serde_json::to_value(&body).ok(),
                    message: None,
                }
            }
            Err(e) => DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "source".to_string(),
                body: None,
                message: Some(format!("Failed to read source file: {}", e)),
            },
        }
    }

    /// Handle exceptionInfo request
    ///
    /// Returns details about the most recent exception (die/croak) encountered
    /// during debugging. Reads from `self.last_exception_message` which is
    /// populated by the output reader when exception patterns are detected.
    pub(super) fn handle_exception_info(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let _args: Option<ExceptionInfoArguments> =
            arguments.and_then(|v| serde_json::from_value(v).ok());

        let stored_message =
            lock_or_recover(&self.last_exception_message, "debug_adapter.last_exception_message");
        let exception_text = stored_message.clone();
        drop(stored_message);

        let body = match exception_text {
            Some(ref message) => ExceptionInfoResponseBody {
                exception_id: "perl_exception".to_string(),
                description: Some(message.clone()),
                break_mode: "always".to_string(),
                details: Some(ExceptionDetails {
                    message: Some(message.clone()),
                    type_name: Some("die".to_string()),
                    stack_trace: None,
                }),
            },
            None => ExceptionInfoResponseBody {
                exception_id: "perl_exception".to_string(),
                description: Some("Unknown exception".to_string()),
                break_mode: "always".to_string(),
                details: None,
            },
        };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "exceptionInfo".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    /// Query `%INC` from the debugger and return parsed (module_key, abs_path) pairs.
    pub(super) fn query_inc_entries(&self) -> Vec<(String, String)> {
        let output_frame_markers = {
            let mut session_guard = lock_or_recover(&self.session, "debug_adapter.session");
            if let Some(ref mut session) = *session_guard {
                if let Some(stdin) = session.process.stdin.as_mut() {
                    let commands = vec!["x \\%INC".to_string()];
                    self.send_framed_debugger_commands(stdin, &commands).ok()
                } else {
                    None
                }
            } else {
                None
            }
        };
        // Session guard dropped — safe to read output.
        let lines = match output_frame_markers {
            Some((begin, end)) => self
                .capture_framed_debugger_output(&begin, &end, DEBUGGER_QUERY_WAIT_MS * 8)
                .unwrap_or_default(),
            None => return Vec::new(),
        };

        let re = match inc_re() {
            Some(re) => re,
            None => return Vec::new(),
        };

        let mut entries = Vec::new();
        for line in &lines {
            if self.cancel_requested.load(Ordering::Acquire) {
                self.cancel_requested.store(false, Ordering::Release);
                return Vec::new();
            }
            if let Some(caps) = re.captures(line) {
                if let (Some(key), Some(val)) = (caps.get(1), caps.get(2)) {
                    entries.push((key.as_str().to_string(), val.as_str().to_string()));
                }
            }
        }
        entries
    }

    /// Handle loadedSources request — returns all files loaded via `%INC`.
    pub(super) fn handle_loaded_sources(
        &self,
        seq: i64,
        request_seq: i64,
        _arguments: Option<Value>,
    ) -> DapMessage {
        let has_session = lock_or_recover(&self.session, "debug_adapter.session").is_some();

        let sources = if has_session {
            self.query_inc_entries()
                .into_iter()
                .map(|(key, path)| crate::protocol::Source { name: Some(key), path: Some(path) })
                .collect()
        } else {
            Vec::new()
        };

        let body = LoadedSourcesResponseBody { sources };
        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "loadedSources".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }

    /// Handle modules request — returns Perl modules from `%INC` with pagination.
    pub(super) fn handle_modules(
        &self,
        seq: i64,
        request_seq: i64,
        arguments: Option<Value>,
    ) -> DapMessage {
        let args: Option<ModulesArguments> = arguments.and_then(|v| serde_json::from_value(v).ok());

        let start_module = args.as_ref().and_then(|a| a.start_module).unwrap_or(0).max(0) as usize;
        let module_count = args.as_ref().and_then(|a| a.module_count);

        let has_session = lock_or_recover(&self.session, "debug_adapter.session").is_some();

        let all_entries = if has_session { self.query_inc_entries() } else { Vec::new() };

        let total = all_entries.len() as i64;

        // Convert Foo/Bar.pm keys to Foo::Bar module names.
        let all_modules: Vec<Module> = all_entries
            .into_iter()
            .enumerate()
            .map(|(idx, (key, path))| {
                let name = module_path_to_name(&key);
                Module { id: idx.to_string(), name, path: Some(path) }
            })
            .collect();

        // Apply pagination.
        let paginated: Vec<Module> = if let Some(count) = module_count {
            all_modules.into_iter().skip(start_module).take(count.max(0) as usize).collect()
        } else {
            all_modules.into_iter().skip(start_module).collect()
        };

        let body = ModulesResponseBody { modules: paginated, total_modules: Some(total) };

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "modules".to_string(),
            body: serde_json::to_value(&body).ok(),
            message: None,
        }
    }
}
