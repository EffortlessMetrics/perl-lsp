//! Output handling: source retrieval, loaded sources, modules, inline values, exception info.

use super::*;
use std::collections::HashSet;

impl DebugAdapter {
    /// Handle inlineValues request (custom)
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
        let validated_path = match self.validate_source_path(&source_path) {
            Ok(path) => path,
            Err(e) => {
                return DapMessage::Response {
                    seq,
                    request_seq,
                    success: false,
                    command: "inlineValues".to_string(),
                    body: None,
                    message: Some(e),
                };
            }
        };

        let content = match std::fs::read_to_string(&validated_path) {
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

        let inline_values = collect_inline_values(&content, start_line, end_line);
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
            if let Some(caps) = re.captures(line)
                && let (Some(key), Some(val)) = (caps.get(1), caps.get(2))
            {
                entries.push((key.as_str().to_string(), val.as_str().to_string()));
            }
        }
        normalize_inc_entries(entries)
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

fn normalize_inc_entries(entries: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut normalized: Vec<_> = entries
        .into_iter()
        .filter(|(key, path)| seen.insert((key.clone(), path.clone())))
        .collect();

    normalized.sort_by(|(left_key, left_path), (right_key, right_path)| {
        module_path_to_name(left_key)
            .cmp(&module_path_to_name(right_key))
            .then_with(|| left_key.cmp(right_key))
            .then_with(|| left_path.cmp(right_path))
    });

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn normalize_inc_entries_sorts_and_deduplicates_results() {
        let entries = vec![
            ("warnings.pm".to_string(), "/app/lib/warnings.pm".to_string()),
            ("Foo/Bar.pm".to_string(), "/app/lib/Foo/Bar.pm".to_string()),
            ("strict.pm".to_string(), "/app/lib/strict.pm".to_string()),
            ("Foo/Bar.pm".to_string(), "/app/lib/Foo/Bar.pm".to_string()),
            ("Foo/Baz.pm".to_string(), "/app/lib/Foo/Baz.pm".to_string()),
        ];

        let normalized = normalize_inc_entries(entries);
        let actual: Vec<_> = normalized.iter().map(|(key, _)| key.as_str()).collect();

        assert_eq!(actual, vec!["Foo/Bar.pm", "Foo/Baz.pm", "strict.pm", "warnings.pm"]);
        assert_eq!(normalized.len(), 4, "duplicate %INC entries should be removed");
    }

    #[test]
    fn inline_values_rejects_paths_outside_workspace_root() -> Result<(), Box<dyn std::error::Error>>
    {
        let adapter = DebugAdapter::new();
        let workspace = tempdir()?;
        let outside = tempdir()?;
        let outside_file = outside.path().join("secret.pl");
        std::fs::write(
            &outside_file,
            "my $secret = 1;
",
        )?;

        *lock_or_recover(&adapter.workspace_root, "tests.workspace_root") =
            Some(workspace.path().to_path_buf());

        let response = adapter.handle_inline_values(
            1,
            1,
            Some(json!({
                "source": { "path": outside_file.to_string_lossy() },
                "startLine": 1,
                "endLine": 1
            })),
        );

        match response {
            DapMessage::Response { success, command, message, .. } => {
                assert!(!success, "inlineValues should reject workspace escapes");
                assert_eq!(command, "inlineValues");
                let message = message.unwrap_or_default();
                assert!(
                    message.contains("Path validation failed"),
                    "unexpected message: {message}"
                );
                Ok(())
            }
            other => Err(format!("expected response, got {other:?}").into()),
        }
    }
}
