//! Variable inspection: variable display, scope variables, set variable.

use super::*;

impl DebugAdapter {
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

        let variables_ref = Self::i64_to_i32_saturating(args.variables_reference);
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

        // AC8.4: Render scalars/arrays/hashes with lazy child expansion.
        let parsed_from_output;
        let mut parsed_child_cache = HashMap::new();
        let mut parsed_full_roots = Vec::new();
        let mut used_session_cache = false;

        if let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session") {
            // Serve requested pages from cache for stable references and cheap repeated expansion.
            if let Some(vars) = session.variable_cache.get_page(variables_ref, start, count) {
                used_session_cache = true;
                parsed_from_output = vars;
            } else {
                let mut framed_scope_lines = None;

                // Request fresh scope output from Perl debugger for scope roots only.
                //
                // Scope encoding: variables_ref % 10 indicates the scope kind:
                //   1 = Locals  (lexical `my` variables in the current frame)
                //   2 = Package (package/`our` variables, fully-qualified names)
                //   3 = Globals (Perl built-in global variables)
                //
                // Frame index (variables_ref / 10) is used only for Package/Globals
                // lookups where `V <pkg>` is appropriate.  For Locals, the `V` command
                // is unsuitable because it only shows package-symbol-table entries —
                // `my` lexicals are NOT in the symbol table.  Instead, we use a
                // B-module eval that walks the current pad directly.
                let frame_id = variables_ref / 10;
                match variables_ref % 10 {
                    1 => {
                        // Locals scope: enumerate lexical `my` variables in the current
                        // executing frame's pad using the B introspection module.
                        //
                        // Why not `V <frame_id> .`?  The `V` command takes a PACKAGE NAME,
                        // not a frame number.  Passing a numeric frame_id (e.g. `V 1 .`)
                        // looks up a package named "1" (which does not exist) and returns
                        // no output.  The subsequent fallback to `fallback_scope_variables`
                        // then returns fake DB-internal placeholders (`$self`, `@_`).
                        //
                        // The B-module eval approach:
                        //   1. Gets the current frame's CV via `$DB::sub` (set by perl5db.pl
                        //      to the sub name when stopped inside a subroutine, undef at
                        //      file scope) or `B::main_cv()` for the file-scope frame.
                        //   2. Walks the pad name list and value list in parallel,
                        //      using `$va[-1]` (the last/innermost pad) so recursive
                        //      calls show the current-innermost frame, not the outermost.
                        //   3. Emits one `$name = value` line per lexical variable,
                        //      which is the same format the `V` command would produce for
                        //      package variables — fully compatible with `parse_scope_variables_from_lines`.
                        //
                        // The outer `eval {}` absorbs any errors (e.g. B not loadable) and
                        // returns an empty string, so the framed output will be empty and
                        // the adapter falls through to `parse_scope_variables_from_output`.
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let cmd = concat!(
                                "p eval { require B; ",
                                "my $cv=$DB::sub?B::svref_2object(\\&{$DB::sub}):B::main_cv(); ",
                                "my $pl=$cv->PADLIST; ",
                                "my @nm=$pl->NAMES->ARRAY; ",
                                "my @va=$pl->ARRAY; ",
                                "my @pds=(@va>1)?$va[-1]->ARRAY:(); ",
                                "my $o=''; ",
                                "for my $i (0..$#nm) { ",
                                "  my $n=$nm[$i]; ",
                                "  next if ref($n) eq 'B::SPECIAL'; ",
                                "  my $pv=eval{$n->PVX}//''; ",
                                "  next unless $pv=~/^[\\$\\@%]/; ",
                                "  my $s=$i<@pds?$pds[$i]:undef; ",
                                "  next unless defined $s; ",
                                "  my $v=eval{$s->SV->PV}//eval{$s->SV->IV}//eval{$s->IV}//eval{$s->PV}//'undef'; ",
                                "  $o.=\"$pv = $v\\n\" ",
                                "} $o }",
                            );
                            let commands = vec![cmd.to_string()];
                            match self.send_framed_debugger_commands(stdin, &commands) {
                                Ok((begin, end)) => {
                                    framed_scope_lines = self.capture_framed_debugger_output(
                                        &begin,
                                        &end,
                                        DEBUGGER_QUERY_WAIT_MS * 8,
                                    );
                                }
                                Err(error) => {
                                    tracing::warn!(%error, "Failed to send framed locals command, falling back");
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

                let (full_roots, child_cache) = if let Some(lines) = framed_scope_lines.as_ref() {
                    let (framed_vars, framed_child_cache) =
                        Self::parse_scope_variables_from_lines(lines, variables_ref, 0, 1024);
                    if framed_vars.is_empty() {
                        Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                        self.parse_scope_variables_from_output(variables_ref, 0, 1024)
                    } else {
                        (framed_vars, framed_child_cache)
                    }
                } else {
                    Self::wait_for_debugger_output_window(DEBUGGER_QUERY_WAIT_MS as u32);
                    self.parse_scope_variables_from_output(variables_ref, 0, 1024)
                };

                parsed_from_output = slice_variables(&full_roots, start, count);
                parsed_full_roots = full_roots;
                parsed_child_cache = child_cache;
            }
        } else {
            let (full_roots, _child_cache) =
                self.parse_scope_variables_from_output(variables_ref, 0, 1024);
            parsed_from_output = slice_variables(&full_roots, start, count);
        }

        let variables = if parsed_from_output.is_empty() {
            Self::fallback_scope_variables(variables_ref, start, count)
        } else {
            parsed_from_output
        };

        // Cache parsed roots and generated child references for expansion/paging requests.
        if !used_session_cache
            && !parsed_full_roots.is_empty()
            && let Some(ref mut session) = *lock_or_recover(&self.session, "debug_adapter.session")
        {
            session.variable_cache.upsert(
                variables_ref,
                VariableCacheKind::Root,
                parsed_full_roots,
            );
            for (reference, children) in parsed_child_cache {
                session.variable_cache.upsert(reference, VariableCacheKind::Child, children);
            }
            let _ = session.variable_cache.get_page(variables_ref, start, count);
        }

        DapMessage::Response {
            seq,
            request_seq,
            success: true,
            command: "variables".to_string(),
            body: Some(json!({
                "variables": variables
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

        if contains_unquoted_statement_separator(value) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some(
                    "setVariable: unsafe value rejected: statement separators are not allowed"
                        .to_string(),
                ),
            };
        }

        if !is_safe_set_variable_value(value) {
            return DapMessage::Response {
                seq,
                request_seq,
                success: false,
                command: "setVariable".to_string(),
                body: None,
                message: Some(
                    "setVariable: unsafe value rejected: only literal or simple variable-reference values are allowed"
                        .to_string(),
                ),
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

        let variables_reference =
            self.allocate_evaluate_result_ref(name, &rendered_value, &rendered_type);
        let set_var_body = SetVariableResponseBody {
            value: rendered_value,
            type_: Some(rendered_type),
            variables_reference,
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

fn contains_unquoted_statement_separator(value: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            ';' if !in_single_quote && !in_double_quote => return true,
            _ => {}
        }
    }

    false
}

fn is_safe_set_variable_value(value: &str) -> bool {
    let value = value.trim();
    value == "undef"
        || is_quoted_literal(value)
        || is_numeric_literal(value)
        || is_valid_set_variable_name(value)
}

fn is_quoted_literal(value: &str) -> bool {
    let Some(quote) = value.chars().next().filter(|ch| *ch == '\'' || *ch == '"') else {
        return false;
    };

    let mut escaped = false;
    for (idx, ch) in value.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == quote {
            return idx + ch.len_utf8() == value.len();
        }
    }

    false
}

fn is_numeric_literal(value: &str) -> bool {
    let normalized: String = value.chars().filter(|ch| *ch != '_').collect();
    let has_digit = normalized.chars().any(|ch| ch.is_ascii_digit());
    let allowed_chars = normalized
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.' | 'e' | 'E'));

    has_digit && allowed_chars && normalized.parse::<f64>().is_ok()
}
