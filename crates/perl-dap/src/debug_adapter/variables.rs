//! Variable inspection: variable display, scope variables, set variable.

use super::*;

impl DebugAdapter {
    /// Handle variables request
    pub fn handle_variables(
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

        // Clamp i64 → i32 safely: values outside [1, i32::MAX] cannot encode a valid scope ref
        // (scope encoding is frame_id * 10 + {1,2,3}, all positive). Negative, zero, or
        // out-of-i32-range refs return protocol-safe empty per DAP spec — success=true, variables=[].
        // We check the raw i64 first to catch huge positive overflow before saturation would
        // hide it (i64::MAX saturates to i32::MAX, which is a non-zero i32 and would pass
        // a simple `== 0` check — wrong). Refs in (0, i32::MAX] are passed to i64_to_i32_saturating.
        let variables_ref_raw = args.variables_reference;
        if variables_ref_raw <= 0 || variables_ref_raw > i32::MAX as i64 {
            // Out-of-range: return protocol-safe empty response immediately.
            return DapMessage::Response {
                seq,
                request_seq,
                success: true,
                command: "variables".to_string(),
                body: Some(json!({ "variables": [] })),
                message: None,
            };
        }
        let variables_ref = Self::i64_to_i32_saturating(variables_ref_raw);

        let start = args.start.unwrap_or(0) as usize;
        let count = args.count.map(|v| v as usize).unwrap_or(256).clamp(1, 1024);

        // Stale-ref guard: if a session exists but the debugger is not stopped, the cache
        // has been cleared (variable_cache.clear() is called on every continue/step). Any
        // variablesReference the client holds from the previous stop is stale. Querying
        // the debugger while it is running would hang or produce garbage. Return the
        // protocol-safe honest empty immediately.
        {
            let session_guard = lock_or_recover(&self.session, "debug_adapter.session");
            if let Some(ref session) = *session_guard {
                if session.state != DebugState::Stopped {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: true,
                        command: "variables".to_string(),
                        body: Some(json!({ "variables": [] })),
                        message: None,
                    };
                }
            }
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
                // Decode the variablesReference using the VariableReference codec.
                // Scope variants map to their kind (Locals/Package/Globals) and frame_id.
                // Non-Scope variants and invalid refs skip the framed output fetch;
                // EvalResult cache hits were already served above.
                use crate::debug_adapter::var_ref::{ScopeKind, VariableReference};

                // Short-circuit: stale EvalResult ref (cache miss after resume).
                //
                // On resume (continue/next/step), variable_cache.clear() runs, making
                // any eval_ref the client holds from the previous stop stale. A stale
                // eval_ref is in the EvalResult band ([1_000_000, 1_999_999_999]) but is
                // absent from the cache. Querying the debugger for a bogus scope or waiting
                // for output that will never arrive is wasteful and semantically wrong.
                //
                // Protocol contract: return honest empty (success=true, variables=[])
                // immediately. This matches the DAP spec — a ref that is no longer valid
                // after a resume simply has no children.
                if matches!(
                    VariableReference::decode(variables_ref),
                    Some(VariableReference::EvalResult { .. })
                ) {
                    return DapMessage::Response {
                        seq,
                        request_seq,
                        success: true,
                        command: "variables".to_string(),
                        body: Some(json!({ "variables": [] })),
                        message: None,
                    };
                }

                let (scope_frame_id, scope_kind) = match VariableReference::decode(variables_ref) {
                    Some(VariableReference::Scope { frame_id, kind }) => (frame_id, Some(kind)),
                    _ => (0, None),
                };
                match scope_kind {
                    Some(ScopeKind::Locals) => {
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
                    Some(ScopeKind::Package) => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} ::", scope_frame_id)];
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
                                    let cmd = format!("V {} ::\n", scope_frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    Some(ScopeKind::Globals) => {
                        if let Some(stdin) = session.process.stdin.as_mut() {
                            let commands = vec![format!("V {} *", scope_frame_id)];
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
                                    let cmd = format!("V {} *\n", scope_frame_id);
                                    let _ = stdin.write_all(cmd.as_bytes());
                                    let _ = stdin.flush();
                                }
                            }
                        }
                    }
                    None => {
                        // Invalid or non-Scope variablesReference — no framed output to fetch.
                        // EvalResult/Child cache hits were already served above;
                        // unknown refs produce an honest empty list.
                    }
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

// ---------------------------------------------------------------------------
// Hazard-class invariant tests (inline lib tests for patch coverage)
//
// These cover three hazard classes from SPEC_UPDATE_CHECKLIST §8:
//   1. Protocol-safety  — invalid/unknown/stale ref → success=true, variables=[]
//   2. Bounds/overflow  — extreme i64 values → checked, no panic/wrap
//   3. ID/ref-space     — eval_ref range (1_000_000+, from #1219) passes through
//                         the invalid-ref guard without being misrouted/rejected
// ---------------------------------------------------------------------------

#[cfg(test)]
mod hazard_invariant_tests {
    use super::*;
    use crate::debug_adapter::DebugAdapter;
    use serde_json::json;

    fn adapter() -> DebugAdapter {
        DebugAdapter::new()
    }

    fn variables_body_is_empty(adapter: &mut DebugAdapter, variables_ref: i64) -> bool {
        let msg = adapter.handle_request(
            1,
            "variables",
            Some(json!({ "variablesReference": variables_ref })),
        );
        match msg {
            DapMessage::Response { success, body, .. } => {
                if !success {
                    return false;
                }
                let vars = body
                    .as_ref()
                    .and_then(|b| b.get("variables"))
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(usize::MAX);
                vars == 0
            }
            _ => false,
        }
    }

    fn variables_success(adapter: &mut DebugAdapter, variables_ref: i64) -> bool {
        match adapter.handle_request(
            1,
            "variables",
            Some(json!({ "variablesReference": variables_ref })),
        ) {
            DapMessage::Response { success, .. } => success,
            _ => false,
        }
    }

    // --- Protocol-safety: ref=0 → empty ---
    #[test]
    fn ref_zero_is_protocol_safe_empty() {
        let mut a = adapter();
        assert!(variables_body_is_empty(&mut a, 0), "ref=0 must return empty variables array");
    }

    // --- Protocol-safety: ref=-1 → empty (negative refs are invalid) ---
    #[test]
    fn ref_negative_is_protocol_safe_empty() {
        let mut a = adapter();
        for bad in [-1_i64, -100, i32::MIN as i64, i64::MIN] {
            assert!(
                variables_body_is_empty(&mut a, bad),
                "ref={bad} (negative) must return empty variables array"
            );
        }
    }

    // --- Bounds/overflow: i64::MAX → no panic, returns empty (> i32::MAX rejected) ---
    #[test]
    fn ref_i64_max_no_panic_returns_empty() {
        let mut a = adapter();
        // i64::MAX saturates to i32::MAX under i64_to_i32_saturating; raw-i64 check rejects it first.
        assert!(
            variables_body_is_empty(&mut a, i64::MAX),
            "ref=i64::MAX must return empty (out-of-i32-range guard)"
        );
        // Just above i32::MAX is also rejected.
        assert!(
            variables_body_is_empty(&mut a, i32::MAX as i64 + 1),
            "ref=i32::MAX+1 must return empty (out-of-range)"
        );
    }

    // --- Bounds/overflow: i32::MAX is in-range (allowed through, no panic) ---
    #[test]
    fn ref_i32_max_no_panic() {
        let mut a = adapter();
        // i32::MAX passes the raw-i64 guard and goes through normal path without session.
        // It must not panic — success=true is required.
        assert!(
            variables_success(&mut a, i32::MAX as i64),
            "ref=i32::MAX must succeed (in-range, no session → honest empty)"
        );
    }

    // --- ID/ref-space: eval_ref range (1_000_000+) is NOT rejected by invalid-ref guard ---
    //
    // #1219 allocates eval refs starting at 1_000_000 (base 1_000_000 + counter).
    // Those refs must pass through the invalid-ref check (0 < 1_000_000 <= i32::MAX) and
    // reach the normal cache-lookup path. Without a session they return honest-empty.
    // Crucially: they must NOT be misrouted as "invalid" just because they look large.
    #[test]
    fn eval_ref_range_passes_invalid_ref_guard() {
        let mut a = adapter();
        // These are valid eval refs from #1219 — not rejected, return success=true.
        for eval_ref in [1_000_000_i64, 1_000_001, 1_000_100, 1_999_999] {
            assert!(
                variables_success(&mut a, eval_ref),
                "eval_ref={eval_ref} (from #1219 range) must not be rejected by invalid-ref guard"
            );
        }
    }

    // --- ID/ref-space: scope refs (frame_id*10 + {1,2,3}) coexist with eval refs ---
    //
    // Scope ref range: frame_id in [0, ~200M] * 10 + {1,2,3} — well below 1_000_000 for
    // reasonable frame counts (< 100_000 frames → refs < 1_000_003). Eval refs start at
    // 1_000_000. Both ranges must produce success=true without session (honest empty).
    #[test]
    fn scope_ref_and_eval_ref_ranges_do_not_collide_for_small_frame_ids() {
        let mut a = adapter();
        // Scope refs for frame_id <=99_999 are in [1, 999_993]; eval refs from #1219
        // start at 1_000_000 — no overlap. Encoding invariant documented in #901/#1219.
        let max_scope_ref: i64 = 99_999 * 10 + 3; // = 999_993
        let min_eval_ref: i64 = 1_000_000;
        // Verify both values PASS the invalid-ref guard ([1, i32::MAX]) and return success.
        // This tests actual guard behavior — the range-non-collision is a precondition
        // documented above, not an assertion on the code under test.
        assert!(
            variables_success(&mut a, max_scope_ref),
            "max scope ref ({max_scope_ref}) must pass invalid-ref guard and succeed"
        );
        assert!(
            variables_success(&mut a, min_eval_ref),
            "min eval ref ({min_eval_ref}) must pass invalid-ref guard and succeed"
        );
    }

    // --- Protocol-safety: never-allocated ref (valid range, but unknown to cache) → no crash ---
    #[test]
    fn never_allocated_ref_in_valid_range_no_crash() {
        let mut a = adapter();
        // A ref that looks like a scope ref but was never actually allocated — no session,
        // so it goes through the no-session path. Must not panic.
        for stale in [11_i64, 12, 13, 21, 22, 23, 999] {
            assert!(
                variables_success(&mut a, stale),
                "never-allocated scope ref={stale} must succeed (no crash, honest empty)"
            );
        }
    }

    // --- Fix #1338: stale EvalResult ref with Stopped session -> early short-circuit ---
    //
    // This lib test covers the new early-return branch in handle_variables() added by
    // fix #1338 (cache-miss branch, EvalResult short-circuit in variables.rs).
    //
    // Path exercised:
    //   1. Session IS Stopped -> passes the Running-state guard (lines 73-92 pre-fix)
    //   2. Cache miss for eval_ref wire -> enters else branch
    //   3. decode yields EvalResult -> short-circuit, return honest empty immediately
    //
    // Without the fix, control falls through to parse_scope_variables_from_output
    // (75ms detour via wait_for_debugger_output_window) before returning empty via
    // fallback_scope_variables. The short-circuit removes the delay and bogus routing.
    //
    // Skip when perl is not on PATH (seed_stopped_session_with_frames_for_test
    // spawns perl -e 1 as a no-op child process).
    #[test]
    fn fix_1338_stale_eval_ref_stopped_session_short_circuits_to_honest_empty() {
        // Skip if perl is not available on PATH.
        if std::process::Command::new("perl").arg("-e").arg("1").output().is_err() {
            return;
        }
        let mut a = adapter();
        // Seed a Stopped session so the Running-state guard does not trigger.
        // This exercises the cache-miss path and the new EvalResult short-circuit.
        a.seed_stopped_session_with_frames_for_test(vec![]);

        // EvalResult band wire values: stale after resume (not in cache).
        for eval_ref_wire in [1_000_000_i64, 1_000_001, 1_000_003, 1_100_000] {
            assert!(
                variables_body_is_empty(&mut a, eval_ref_wire),
                "fix #1338: stopped session + stale eval_ref={eval_ref_wire} must return honest empty"
            );
        }
    }

    // --- Guard test: cached EvalResult is NOT short-circuited by the fix #1338 early return ---
    //
    // This is the scoping guard for the fix: a VALID EvalResult ref that IS in the cache
    // must be served via the cache-hit path (line 102) and return its children — it must
    // NOT be swallowed by the early-return short-circuit (which fires only on cache miss).
    //
    // Without this guard, a regression could incorrectly apply the early return to ALL
    // EvalResult refs (cached or not), causing legitimate variable expansion to return
    // empty. This test would fail immediately in that case.
    //
    // Skip when perl is not on PATH.
    #[test]
    fn fix_1338_cached_eval_result_is_served_not_short_circuited() {
        // Skip if perl is not available on PATH.
        if std::process::Command::new("perl").arg("-e").arg("1").output().is_err() {
            return;
        }
        use crate::debug_adapter::var_ref::VariableReference;
        use crate::types::Variable;

        let mut a = adapter();
        a.seed_stopped_session_with_frames_for_test(vec![]);

        // An EvalResult wire value that IS in cache (simulates a fresh evaluate result
        // before resume — the client holds the ref and sends a variables request while
        // the session is still stopped at the same breakpoint).
        let eval_ref_wire: i32 =
            VariableReference::EvalResult { counter: 42 }.encode().expect("counter=42 is valid");
        assert!(
            (1_000_000..=1_999_999_999).contains(&eval_ref_wire),
            "setup: must be in EvalResult band"
        );

        // Seed a cached entry so the cache-hit path fires.
        let cached_var = Variable {
            name: "".to_string(),
            value: "42".to_string(),
            type_: Some("SCALAR".to_string()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        };
        a.seed_eval_result_cache_for_test(eval_ref_wire, vec![cached_var]);

        // Must return the cached children (non-empty), NOT the early-return empty.
        // If the early return incorrectly fired here, this assertion would fail.
        let msg = a.handle_request(
            1,
            "variables",
            Some(serde_json::json!({ "variablesReference": eval_ref_wire as i64 })),
        );
        match msg {
            DapMessage::Response { success, body, .. } => {
                assert!(success, "cached EvalResult must succeed");
                let vars = body
                    .as_ref()
                    .and_then(|b| b.get("variables"))
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                assert_eq!(
                    vars, 1,
                    "cached EvalResult must return its 1 cached child; got {vars} (early return was applied incorrectly)"
                );
            }
            other => panic!("expected Response, got: {other:?}"),
        }
    }
}
