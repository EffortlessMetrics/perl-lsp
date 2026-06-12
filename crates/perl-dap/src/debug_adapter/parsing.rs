//! Debugger output parsing: normalize, infer types, parse stack frames, parse variables.

mod scope_variables;

use super::*;

impl DebugAdapter {
    /// Normalize debugger output lines for deterministic parsing by:
    /// - removing ANSI escape sequences
    /// - stripping all debugger prompt prefixes (e.g. `DB<1>`, `DB<2>`)
    ///
    /// A single output line from `perl -d` may contain multiple consecutive
    /// prompt tokens when the debugger processes several commands between stops
    /// (e.g. `  DB<1>   DB<2> main::(/path/file.pl:5):`).  All prompts must
    /// be stripped so that the context pattern can match the tail of the line.
    pub(super) fn normalize_debugger_output_line(line: &str) -> String {
        let mut normalized = if let Some(re) = ansi_escape_re() {
            re.replace_all(line, "").into_owned()
        } else {
            line.to_string()
        };

        // Strip all occurrences of DB<N> prompt tokens from the line.
        while let Some(prompt_start) = normalized.find("DB<")
            && let Some(prompt_end) = normalized[prompt_start..].find('>')
        {
            let content_start = prompt_start + prompt_end + 1;
            normalized = normalized[content_start..].to_string();
        }

        normalized.trim().to_string()
    }

    /// Infer a coarse DAP value type from literal-like debugger output.
    pub(super) fn infer_debugger_value_type(text: &str) -> String {
        if text == "undef" {
            "undef".to_string()
        } else if text.parse::<i64>().is_ok() {
            "integer".to_string()
        } else if text.parse::<f64>().is_ok() {
            "number".to_string()
        } else if text.starts_with('[') && text.ends_with(']') {
            "array".to_string()
        } else if text.starts_with('{') && text.ends_with('}') {
            "hash".to_string()
        } else {
            "string".to_string()
        }
    }

    /// Convert microcrate rendered variables into adapter-local protocol values.
    pub(super) fn rendered_to_variable(rendered: RenderedVariable) -> Variable {
        Variable {
            name: rendered.name,
            value: rendered.value,
            type_: rendered.type_name,
            variables_reference: Self::i64_to_i32_saturating(rendered.variables_reference),
            named_variables: rendered.named_variables.map(Self::i64_to_i32_saturating),
            indexed_variables: rendered.indexed_variables.map(Self::i64_to_i32_saturating),
        }
    }

    /// Determine if a variable name should appear in a given scope.
    pub(super) fn scope_allows_variable_name(scope_type: i32, name: &str) -> bool {
        match scope_type {
            // Locals
            1 => !name.contains("::"),
            // Package variables (qualified)
            2 => name.contains("::"),
            // Globals/specials
            3 => {
                matches!(name, "$_" | "@ARGV" | "%ENV" | "$!" | "$@" | "$/" | "$|" | "$0" | "$^W")
                    || name.starts_with("$^")
            }
            _ => true,
        }
    }

    /// Convert parsed stack frames from `perl-dap-stack` into local DAP response frames.
    pub(super) fn parse_stack_frames_from_text(output: &str) -> Vec<StackFrame> {
        let mut parser = PerlStackParser::new();
        parser
            .parse_stack_trace(output)
            .into_iter()
            .map(|frame| {
                let source = frame.source.unwrap_or_default();
                let path = source.path.unwrap_or_else(|| "<unknown>".to_string());
                let name = source.name.or_else(|| {
                    std::path::Path::new(&path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(ToString::to_string)
                });
                StackFrame {
                    id: Self::i64_to_i32_saturating(frame.id),
                    name: frame.name,
                    source: Source { name, path, source_reference: None },
                    line: Self::i64_to_i32_saturating(frame.line),
                    column: Self::i64_to_i32_saturating(frame.column),
                    end_line: frame.end_line.map(Self::i64_to_i32_saturating),
                    end_column: frame.end_column.map(Self::i64_to_i32_saturating),
                }
            })
            .collect()
    }

    /// Filter out internal debugger and shim frames from user-visible stack traces.
    pub(super) fn filter_user_visible_frames(frames: Vec<StackFrame>) -> Vec<StackFrame> {
        frames
            .into_iter()
            .filter(|frame| {
                !is_internal_frame_name_and_path(&frame.name, Some(frame.source.path.as_str()))
            })
            .collect()
    }

    /// Parse variables from debugger output lines using microcrate parser/renderer.
    pub(super) fn parse_scope_variables_from_lines(
        lines: &[String],
        variables_ref: i32,
        start: usize,
        count: usize,
    ) -> (Vec<Variable>, HashMap<i32, Vec<Variable>>) {
        let scope_type = variables_ref % 10;
        let parsed = scope_variables::parse_assignments(lines, scope_type);
        let page = scope_variables::sort_and_paginate(parsed, start, count);

        let mut top_level = Vec::with_capacity(page.len());
        let mut child_cache = HashMap::new();
        for (idx, (name, value)) in page.into_iter().enumerate() {
            let child_ref = scope_variables::compute_child_reference(variables_ref, start, idx);
            let (top, cache_entry) = scope_variables::render_paged_variable(name, value, child_ref);
            top_level.push(top);
            if let Some((k, v)) = cache_entry {
                child_cache.insert(k, v);
            }
        }
        (top_level, child_cache)
    }

    /// Parse variables from recent debugger output using microcrate parser/renderer.
    pub(super) fn parse_scope_variables_from_output(
        &self,
        variables_ref: i32,
        start: usize,
        count: usize,
    ) -> (Vec<Variable>, HashMap<i32, Vec<Variable>>) {
        let lines = self.snapshot_recent_output_lines();
        Self::parse_scope_variables_from_lines(&lines, variables_ref, start, count)
    }

    /// Parse evaluate output from debugger lines into a DAP result payload.
    pub(super) fn parse_evaluate_result_from_lines(
        lines: &[String],
        expression: &str,
        allow_fallback_line: bool,
    ) -> Option<(String, String)> {
        if lines.is_empty() {
            return None;
        }

        let parser = VariableParser::new();
        let renderer = PerlVariableRenderer::new();

        for line in lines.iter().rev() {
            let normalized = Self::normalize_debugger_output_line(line);
            let text = normalized.trim();
            if text.is_empty() || prompt_re().is_some_and(|re| re.is_match(text)) {
                continue;
            }

            if let Ok((name, value)) = parser.parse_assignment(text) {
                let rendered = renderer.render(&name, &value);
                let type_name = rendered.type_name.unwrap_or_else(|| "string".to_string());
                // Prefer direct matches for the evaluated expression, but allow fallback assignment.
                if name == expression || text.starts_with(expression) || text.contains(expression) {
                    return Some((rendered.value, type_name));
                }
                if !allow_fallback_line {
                    continue;
                }
                return Some((rendered.value, type_name));
            }

            if allow_fallback_line {
                return Some((text.to_string(), Self::infer_debugger_value_type(text)));
            }
        }

        None
    }

    /// Parse explicit debugger error lines from evaluate output.
    pub(super) fn parse_evaluate_error_from_lines(lines: &[String]) -> Option<String> {
        const ERROR_PREFIXES: &[&str] =
            &["Undefined", "Can't ", "syntax error", "Execution of ", "Use of uninitialized"];

        for line in lines.iter().rev() {
            let normalized = Self::normalize_debugger_output_line(line);
            let text = normalized.trim();
            if text.is_empty() || prompt_re().is_some_and(|re| re.is_match(text)) {
                continue;
            }

            if ERROR_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
                return Some(format!("evaluate failed: {text}"));
            }
        }

        None
    }

    /// Parse evaluate output from recent debugger lines into a DAP result payload.
    pub(super) fn parse_evaluate_result_from_output(
        &self,
        expression: &str,
    ) -> Option<(String, String)> {
        let lines = self.snapshot_recent_output_lines();
        Self::parse_evaluate_result_from_lines(&lines, expression, true)
    }

    /// Build deterministic placeholder variables used when debugger output is unavailable.
    pub(super) fn fallback_scope_variables(
        variables_ref: i32,
        start: usize,
        count: usize,
    ) -> Vec<Variable> {
        let variables = match variables_ref % 10 {
            1 => vec![
                Variable {
                    name: "$self".to_string(),
                    value: "blessed(My::Module)".to_string(),
                    type_: Some("hash".to_string()),
                    variables_reference: variables_ref.saturating_mul(100) + 2,
                    named_variables: Some(5),
                    indexed_variables: None,
                },
                Variable {
                    name: "@_".to_string(),
                    value: "array(size=0)".to_string(),
                    type_: Some("array".to_string()),
                    variables_reference: variables_ref.saturating_mul(100) + 1,
                    named_variables: None,
                    indexed_variables: Some(0),
                },
            ],
            2 => vec![Variable {
                name: "$VERSION".to_string(),
                value: "\"1.0.0\"".to_string(),
                type_: Some("scalar".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            }],
            3 => vec![Variable {
                name: "$_".to_string(),
                value: "undef".to_string(),
                type_: Some("scalar".to_string()),
                variables_reference: 0,
                named_variables: None,
                indexed_variables: None,
            }],
            _ => Vec::new(),
        };

        variables.into_iter().skip(start).take(count).collect()
    }

    /// Parse stack trace output from Perl debugger "T" command
    ///
    /// AC8.2: Parse caller() + %DB::sub data from Perl debugger
    ///
    /// The Perl debugger "T" command outputs stack traces in formats like:
    /// ```text
    /// $ = main::compute_sum() called from file /app/main.pl line 15
    /// $ = main::process_data() called from file /app/main.pl line 10
    /// ```
    ///
    /// Or with frame numbers:
    /// ```text
    /// # 0 main::helper at /app/script.pl line 20
    /// # 1 Foo::bar called at /app/lib/Foo.pm line 15
    /// # 2 main::start at /app/script.pl line 5
    /// ```
    ///
    /// Returns a vector of StackFrame structs with accurate line numbers,
    /// source paths, and package-qualified function names.
    #[cfg(test)]
    pub(super) fn parse_stack_trace(output: &str) -> Vec<StackFrame> {
        let mut frames = Vec::new();
        let mut frame_id = 1;

        for line in output.lines() {
            // Try to match stack frame format
            if let Some(re) = stack_frame_re() {
                if let Some(caps) = re.captures(line) {
                    let func = caps.name("func").map(|m| m.as_str()).unwrap_or("main");
                    let file = caps.name("file").map(|m| m.as_str()).unwrap_or("<unknown>");
                    let line_num =
                        caps.name("line").and_then(|m| m.as_str().parse::<i32>().ok()).unwrap_or(1);

                    // Extract file name from path for display
                    let file_name = file.split(['/', '\\'].as_ref()).next_back().unwrap_or(file);

                    frames.push(StackFrame {
                        id: frame_id,
                        name: func.to_string(),
                        source: Source {
                            name: Some(file_name.to_string()),
                            path: file.to_string(),
                            source_reference: None,
                        },
                        line: line_num,
                        column: 1, // Perl debugger doesn't provide column info by default
                        end_line: None,
                        end_column: None,
                    });

                    frame_id += 1;
                }
            }
        }

        frames
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    pub(super) fn test_parse_scope_variables_from_recent_output()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        adapter.push_recent_output_line_for_test("$foo = 42");
        adapter.push_recent_output_line_for_test("@arr = (1, 2, 3)");
        adapter.push_recent_output_line_for_test("%hash = {a => 1}");

        let (vars, child_cache) = adapter.parse_scope_variables_from_output(11, 0, 20);
        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"$foo"));
        assert!(names.contains(&"@arr"));
        assert!(names.contains(&"%hash"));
        assert!(!child_cache.is_empty(), "expected child cache entries for expandable values");
        Ok(())
    }

    #[test]
    pub(super) fn test_parse_scope_variables_are_sorted_for_stability()
    -> Result<(), Box<dyn std::error::Error>> {
        let lines = vec!["$zeta = 1".to_string(), "$alpha = 2".to_string(), "$mid = 3".to_string()];

        let (vars, _child_cache) =
            DebugAdapter::parse_scope_variables_from_lines(&lines, 11, 0, 20);
        let names = vars.iter().map(|v| v.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names, vec!["$alpha", "$mid", "$zeta"]);
        Ok(())
    }

    #[test]
    pub(super) fn test_parse_scope_variables_child_refs_stable_across_pages()
    -> Result<(), Box<dyn std::error::Error>> {
        let lines = vec![
            "@alpha = (1, 2)".to_string(),
            "@beta = (3, 4)".to_string(),
            "@gamma = (5, 6)".to_string(),
        ];

        let (page_one, page_one_children) =
            DebugAdapter::parse_scope_variables_from_lines(&lines, 11, 0, 1);
        let (page_two, page_two_children) =
            DebugAdapter::parse_scope_variables_from_lines(&lines, 11, 1, 1);

        let first_ref = page_one
            .first()
            .map(|variable| variable.variables_reference)
            .ok_or("expected first page variable")?;
        let second_ref = page_two
            .first()
            .map(|variable| variable.variables_reference)
            .ok_or("expected second page variable")?;

        assert_ne!(first_ref, second_ref, "paged variables must not reuse child references");
        assert!(
            page_one_children.contains_key(&first_ref),
            "expected first page child cache for first reference"
        );
        assert!(
            page_two_children.contains_key(&second_ref),
            "expected second page child cache for second reference"
        );
        Ok(())
    }

    #[test]
    pub(super) fn test_capture_framed_debugger_output_isolated_by_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        adapter.push_recent_output_line_for_test("noise");
        adapter.push_recent_output_line_for_test(r#""DAP_BEGIN_100""#);
        adapter.push_recent_output_line_for_test("$a = 1");
        adapter.push_recent_output_line_for_test(r#""DAP_END_100""#);
        adapter.push_recent_output_line_for_test(r#""DAP_BEGIN_200""#);
        adapter.push_recent_output_line_for_test("$b = 2");
        adapter.push_recent_output_line_for_test(r#""DAP_END_200""#);

        let lines = adapter
            .capture_framed_debugger_output("DAP_BEGIN_200", "DAP_END_200", 200)
            .ok_or("expected framed output for marker 200")?;
        assert_eq!(lines, vec!["$b = 2".to_string()]);
        Ok(())
    }

    #[test]
    pub(super) fn test_capture_framed_debugger_output_handles_partial_marker_arrival()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        let recent_output = adapter.recent_output.clone();
        let producer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(DEBUGGER_FRAME_POLL_MS * 2));
            let mut output = lock_or_recover(&recent_output, "test_partial_marker.recent_output");
            DebugAdapter::append_recent_output_line_locked(&mut output, r#""DAP_BEGIN_300""#);
            DebugAdapter::append_recent_output_line_locked(&mut output, "interleaved noise");
            DebugAdapter::append_recent_output_line_locked(&mut output, "$captured = 42");
            DebugAdapter::append_recent_output_line_locked(&mut output, r#""DAP_END_300""#);
        });

        let lines = adapter
            .capture_framed_debugger_output("DAP_BEGIN_300", "DAP_END_300", 500)
            .ok_or("expected framed output for delayed markers")?;
        producer.join().map_err(|_| "producer thread panicked")?;
        assert_eq!(lines, vec!["interleaved noise".to_string(), "$captured = 42".to_string()]);
        Ok(())
    }

    #[test]
    pub(super) fn test_capture_framed_debugger_output_respects_cancellation() {
        let adapter = DebugAdapter::new();
        adapter.cancel_requested.store(true, Ordering::Release);

        let capture = adapter.capture_framed_debugger_output("DAP_BEGIN_400", "DAP_END_400", 200);
        assert!(capture.is_none(), "capture should stop when request is cancelled");
        assert!(
            !adapter.cancel_requested.load(Ordering::Acquire),
            "cancellation flag should be reset after capture returns"
        );
    }

    #[test]
    pub(super) fn test_capture_framed_debugger_output_timeout_without_end_marker() {
        let adapter = DebugAdapter::new();
        adapter.push_recent_output_line_for_test(r#""DAP_BEGIN_500""#);
        adapter.push_recent_output_line_for_test("$value = 1");

        let start = Instant::now();
        let capture = adapter.capture_framed_debugger_output("DAP_BEGIN_500", "DAP_END_500", 1);
        assert!(capture.is_none(), "capture should timeout without end marker");
        assert!(
            start.elapsed() >= Duration::from_millis(DEBUGGER_QUERY_WAIT_MS),
            "timeout should honor minimum debugger query budget"
        );
    }

    #[test]
    pub(super) fn test_framed_capture_marker_scan_microbenchmark() {
        let mut lines = Vec::with_capacity(RECENT_OUTPUT_MAX_LINES);
        for idx in 0..(RECENT_OUTPUT_MAX_LINES - 4) {
            let raw = format!("DB<1> noise line {idx}");
            lines.push(RecentOutputLine {
                id: idx as u64 + 1,
                normalized: DebugAdapter::normalize_debugger_output_line(&raw),
                raw,
            });
        }
        let begin_id = RECENT_OUTPUT_MAX_LINES as u64 - 3;
        lines.push(RecentOutputLine {
            id: begin_id,
            raw: r#""DAP_BEGIN_900""#.to_string(),
            normalized: r#""DAP_BEGIN_900""#.to_string(),
        });
        lines.push(RecentOutputLine {
            id: begin_id + 1,
            raw: "$x = 1".to_string(),
            normalized: "$x = 1".to_string(),
        });
        lines.push(RecentOutputLine {
            id: begin_id + 2,
            raw: "$y = 2".to_string(),
            normalized: "$y = 2".to_string(),
        });
        lines.push(RecentOutputLine {
            id: begin_id + 3,
            raw: r#""DAP_END_900""#.to_string(),
            normalized: r#""DAP_END_900""#.to_string(),
        });

        let iterations = 300;
        let full_scan_start = Instant::now();
        for _ in 0..iterations {
            let normalized = lines
                .iter()
                .map(|line| DebugAdapter::normalize_debugger_output_line(&line.raw))
                .collect::<Vec<_>>();
            let _ = normalized.iter().rposition(|line| line.contains("DAP_BEGIN_900")).and_then(
                |begin_idx| {
                    normalized[begin_idx + 1..]
                        .iter()
                        .position(|line| line.contains("DAP_END_900"))
                        .map(|end_rel| normalized[begin_idx + 1..begin_idx + 1 + end_rel].len())
                },
            );
        }
        let full_scan_elapsed = full_scan_start.elapsed();

        let incremental_start = Instant::now();
        for _ in 0..iterations {
            let mut saw_begin = false;
            for line in &lines {
                if !saw_begin {
                    if DebugAdapter::line_contains_full_marker(&line.normalized, "DAP_BEGIN_900") {
                        saw_begin = true;
                    }
                } else if DebugAdapter::line_contains_full_marker(&line.normalized, "DAP_END_900") {
                    break;
                }
            }
        }
        let incremental_elapsed = incremental_start.elapsed();

        assert!(
            incremental_elapsed < full_scan_elapsed,
            "expected incremental scan ({incremental_elapsed:?}) to be faster than full scan ({full_scan_elapsed:?})"
        );
    }

    #[test]
    pub(super) fn test_stack_trace_returns_empty_without_live_session()
    -> Result<(), Box<dyn std::error::Error>> {
        // After fix #933: the degraded-transport path no longer parses the snapshot
        // buffer. When there is no live session, handle_stack_trace returns an empty
        // frame list regardless of recent-output content (snapshot buffer is unreliable
        // because it contains full session history in arrival order).
        let mut adapter = DebugAdapter::new();
        adapter.push_recent_output_line_for_test("# 0 main::compute at /tmp/script.pl line 20");
        adapter.push_recent_output_line_for_test("# 1 Foo::process called at /tmp/Foo.pm line 15");

        let response = adapter.handle_request(1, "stackTrace", Some(json!({"threadId": 1})));
        match response {
            DapMessage::Response { success, body, .. } => {
                assert!(success);
                let body = body.ok_or("missing stackTrace body")?;
                let frames = body
                    .get("stackFrames")
                    .and_then(|v| v.as_array())
                    .ok_or("missing stackFrames")?;
                // No live session → degraded path returns Vec::new() → session.stack_frames
                // (also empty) → honest empty list per DAP spec.
                assert_eq!(
                    frames.len(),
                    0,
                    "without a live session, snapshot buffer must NOT be used; expected 0 frames, got {}",
                    frames.len()
                );
            }
            _ => return Err("expected stackTrace response".into()),
        }
        Ok(())
    }

    #[test]
    pub(super) fn test_parse_evaluate_result_from_recent_output()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = DebugAdapter::new();
        adapter.push_recent_output_line_for_test("$result = 123");

        let parsed = adapter.parse_evaluate_result_from_output("$result");
        let (value, ty) = parsed.ok_or("expected parsed evaluate result")?;
        assert_eq!(value, "123");
        assert_eq!(ty, "SCALAR");
        Ok(())
    }

    /// Helper to create a test stack frame
    pub(super) fn make_test_frame(id: i32, name: &str, path: &str, line: i32) -> StackFrame {
        StackFrame {
            id,
            name: name.to_string(),
            source: Source {
                name: Some(path.split('/').next_back().unwrap_or(path).to_string()),
                path: path.to_string(),
                source_reference: None,
            },
            line,
            column: 1,
            end_line: None,
            end_column: None,
        }
    }

    /// Test helper: Filter frames using the same logic as handle_stack_trace (AC8.2.1)
    pub(super) fn filter_internal_frames(frames: Vec<StackFrame>) -> Vec<StackFrame> {
        DebugAdapter::filter_user_visible_frames(frames)
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_removes_db_frames() {
        // AC8.2.1: Filter internal frames from user-visible stack
        let frames = vec![
            make_test_frame(1, "main::hello", "/app/hello.pl", 10),
            make_test_frame(2, "DB::DB", "/usr/share/perl/5.34/perl5db.pl", 100),
            make_test_frame(3, "Foo::bar", "/app/lib/Foo.pm", 25),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::hello");
        assert_eq!(filtered[1].name, "Foo::bar");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_removes_shim_frames() {
        // AC8.2.1: Filter Devel::TSPerlDAP:: shim infrastructure frames
        let frames = vec![
            make_test_frame(1, "Devel::TSPerlDAP::init", "/shim/TSPerlDAP.pm", 50),
            make_test_frame(2, "main::run", "/app/script.pl", 5),
            make_test_frame(3, "Devel::TSPerlDAP::handle_break", "/shim/TSPerlDAP.pm", 200),
            make_test_frame(4, "Utils::process", "/app/lib/Utils.pm", 42),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::run");
        assert_eq!(filtered[1].name, "Utils::process");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_removes_perl5db_source() {
        // AC8.2.1: Filter frames from perl5db.pl even with different names
        let frames = vec![
            make_test_frame(1, "main::start", "/app/main.pl", 1),
            make_test_frame(2, "some_internal", "/usr/lib/perl5/perl5db.pl", 999),
            make_test_frame(3, "App::process", "/app/lib/App.pm", 100),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::start");
        assert_eq!(filtered[1].name, "App::process");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_mixed_internal_frames() {
        // AC8.2.1: Comprehensive test with all types of internal frames mixed
        let frames = vec![
            // User frame 1
            make_test_frame(1, "main::hello", "/app/hello.pl", 10),
            // DB:: frame (internal)
            make_test_frame(2, "DB::sub", "/usr/share/perl/5.34/perl5db.pl", 2000),
            // User frame 2
            make_test_frame(3, "Foo::bar", "/app/lib/Foo.pm", 25),
            // Shim frame (internal)
            make_test_frame(4, "Devel::TSPerlDAP::step", "/shim/TSPerlDAP.pm", 150),
            // DB:: frame without perl5db.pl path (still filtered)
            make_test_frame(5, "DB::breakpoint", "/some/other/path.pm", 50),
            // User frame 3
            make_test_frame(6, "Baz::qux", "/app/lib/Baz.pm", 75),
            // perl5db.pl source frame (internal)
            make_test_frame(7, "custom_handler", "/usr/lib/perl5/perl5db.pl", 1500),
        ];

        let filtered = filter_internal_frames(frames);

        // Should only have user frames: main::hello, Foo::bar, Baz::qux
        assert_eq!(filtered.len(), 3, "Expected 3 user frames, got {}", filtered.len());
        assert_eq!(filtered[0].name, "main::hello");
        assert_eq!(filtered[1].name, "Foo::bar");
        assert_eq!(filtered[2].name, "Baz::qux");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_preserves_order() {
        // Verify frame order is preserved after filtering
        let frames = vec![
            make_test_frame(1, "A::first", "/a.pm", 1),
            make_test_frame(2, "DB::internal", "/perl5db.pl", 100),
            make_test_frame(3, "B::second", "/b.pm", 2),
            make_test_frame(4, "Devel::TSPerlDAP::shim", "/shim.pm", 50),
            make_test_frame(5, "C::third", "/c.pm", 3),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "A::first");
        assert_eq!(filtered[1].name, "B::second");
        assert_eq!(filtered[2].name, "C::third");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_all_internal() {
        // Edge case: all frames are internal
        let frames = vec![
            make_test_frame(1, "DB::main", "/perl5db.pl", 1),
            make_test_frame(2, "Devel::TSPerlDAP::init", "/shim.pm", 10),
            make_test_frame(3, "DB::sub", "/perl5db.pl", 50),
        ];

        let filtered = filter_internal_frames(frames);

        assert!(filtered.is_empty(), "Expected empty stack after filtering all internal frames");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_no_internal() {
        // Edge case: no internal frames to filter
        let frames = vec![
            make_test_frame(1, "main::start", "/app/main.pl", 1),
            make_test_frame(2, "Lib::helper", "/app/lib/Lib.pm", 50),
            make_test_frame(3, "Utils::format", "/app/lib/Utils.pm", 100),
        ];

        let filtered = filter_internal_frames(frames);

        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "main::start");
        assert_eq!(filtered[1].name, "Lib::helper");
        assert_eq!(filtered[2].name, "Utils::format");
    }

    #[test]
    pub(super) fn test_stack_frame_filtering_empty_input() {
        // Edge case: empty frame list
        let frames: Vec<StackFrame> = vec![];
        let filtered = filter_internal_frames(frames);
        assert!(filtered.is_empty());
    }

    // AC8.2.4: Stack trace parsing tests for simple call chains (A → B → C)
    #[test]
    pub(super) fn test_parse_stack_trace_simple_call_chain() {
        let output = r#"# 0 main::compute_sum at /app/script.pl line 20
# 1 Foo::process called at /app/lib/Foo.pm line 15
# 2 main::start at /app/script.pl line 5"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 3);

        // Frame 0: main::compute_sum
        assert_eq!(frames[0].id, 1);
        assert_eq!(frames[0].name, "main::compute_sum");
        assert_eq!(frames[0].source.path, "/app/script.pl");
        assert_eq!(frames[0].line, 20);
        assert_eq!(frames[0].source.name, Some("script.pl".to_string()));

        // Frame 1: Foo::process
        assert_eq!(frames[1].id, 2);
        assert_eq!(frames[1].name, "Foo::process");
        assert_eq!(frames[1].source.path, "/app/lib/Foo.pm");
        assert_eq!(frames[1].line, 15);

        // Frame 2: main::start
        assert_eq!(frames[2].id, 3);
        assert_eq!(frames[2].name, "main::start");
        assert_eq!(frames[2].source.path, "/app/script.pl");
        assert_eq!(frames[2].line, 5);
    }

    // AC8.2.4: Stack trace parsing for multi-file call stacks across packages
    #[test]
    pub(super) fn test_parse_stack_trace_multi_file_packages() {
        let output = r#"# 0 Utils::Helper::validate at /app/lib/Utils/Helper.pm line 42
# 1 Data::Processor::transform called at /app/lib/Data/Processor.pm line 120
# 2 Controller::API::handle_request at /app/controller/API.pm line 78
# 3 main::dispatch called at /app/app.pl line 10"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 4);
        assert_eq!(frames[0].name, "Utils::Helper::validate");
        assert_eq!(frames[1].name, "Data::Processor::transform");
        assert_eq!(frames[2].name, "Controller::API::handle_request");
        assert_eq!(frames[3].name, "main::dispatch");

        // Verify cross-file navigation info is present
        assert!(frames[0].source.path.contains("Utils/Helper.pm"));
        assert!(frames[1].source.path.contains("Data/Processor.pm"));
        assert!(frames[2].source.path.contains("controller/API.pm"));
        assert!(frames[3].source.path.contains("app.pl"));
    }

    // AC8.2.4: Stack trace parsing for recursive calls with depth
    #[test]
    pub(super) fn test_parse_stack_trace_recursive_calls() {
        let output = r#"# 0 main::factorial at /app/math.pl line 5
# 1 main::factorial called at /app/math.pl line 6
# 2 main::factorial called at /app/math.pl line 6
# 3 main::factorial called at /app/math.pl line 6
# 4 main::compute at /app/math.pl line 10"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 5);

        // Verify recursive frames are all parsed correctly
        assert_eq!(frames[0].name, "main::factorial");
        assert_eq!(frames[1].name, "main::factorial");
        assert_eq!(frames[2].name, "main::factorial");
        assert_eq!(frames[3].name, "main::factorial");
        assert_eq!(frames[4].name, "main::compute");

        // Verify frame IDs are sequential
        assert_eq!(frames[0].id, 1);
        assert_eq!(frames[1].id, 2);
        assert_eq!(frames[2].id, 3);
        assert_eq!(frames[3].id, 4);
        assert_eq!(frames[4].id, 5);
    }

    // AC8.2.4: Stack trace parsing for anonymous subroutines
    #[test]
    pub(super) fn test_parse_stack_trace_anonymous_subs() {
        let output = r#"# 0 main::__ANON__ at /app/callback.pl line 15
# 1 Utils::map called at /app/lib/Utils.pm line 42
# 2 main::process_items at /app/callback.pl line 10"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 3);

        // Verify anonymous sub is parsed (Perl uses __ANON__ for anonymous subs)
        assert_eq!(frames[0].name, "main::__ANON__");
        assert_eq!(frames[1].name, "Utils::map");
        assert_eq!(frames[2].name, "main::process_items");
    }

    // AC8.2: Stack trace parsing with Windows paths
    #[test]
    pub(super) fn test_parse_stack_trace_windows_paths() {
        let output = r#"# 0 main::test at C:\workspace\script.pl line 10
# 1 Foo::bar called at C:\workspace\lib\Foo.pm line 25"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].source.path, r"C:\workspace\script.pl");
        assert_eq!(frames[0].source.name, Some("script.pl".to_string()));
        assert_eq!(frames[1].source.path, r"C:\workspace\lib\Foo.pm");
        assert_eq!(frames[1].source.name, Some("Foo.pm".to_string()));
    }

    #[test]
    pub(super) fn test_parse_stack_trace_with_space_in_paths() {
        let output = r#"# 0 main::test at /tmp/My Project/script.pl line 10
# 1 Foo::bar called at C:\Work Files\lib\Foo.pm line 25"#;

        let frames = DebugAdapter::parse_stack_trace(output);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].source.path, "/tmp/My Project/script.pl");
        assert_eq!(frames[0].source.name, Some("script.pl".to_string()));
        assert_eq!(frames[1].source.path, r"C:\Work Files\lib\Foo.pm");
        assert_eq!(frames[1].source.name, Some("Foo.pm".to_string()));
    }

    // AC8.2: Stack trace parsing with empty output
    #[test]
    pub(super) fn test_parse_stack_trace_empty_output() {
        let output = "";
        let frames = DebugAdapter::parse_stack_trace(output);
        assert!(frames.is_empty());
    }

    // AC8.2: Stack trace parsing with malformed output
    #[test]
    pub(super) fn test_parse_stack_trace_malformed_output() {
        let output = r#"Random output that doesn't match
Some error message
DB<1>"#;

        let frames = DebugAdapter::parse_stack_trace(output);
        assert!(frames.is_empty());
    }

    // AC8.2.1: Integration test - parse and filter combined
    #[test]
    pub(super) fn test_parse_and_filter_stack_trace() {
        let output = r#"# 0 main::user_func at /app/script.pl line 10
# 1 DB::DB called at /usr/share/perl/5.34/perl5db.pl line 100
# 2 Foo::process at /app/lib/Foo.pm line 25
# 3 Devel::TSPerlDAP::handle_break called at /shim/TSPerlDAP.pm line 50
# 4 main::start at /app/script.pl line 5"#;

        let frames = DebugAdapter::parse_stack_trace(output);
        assert_eq!(frames.len(), 5);

        // Apply filtering
        let filtered = filter_internal_frames(frames);

        // Should only have user frames after filtering
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].name, "main::user_func");
        assert_eq!(filtered[1].name, "Foo::process");
        assert_eq!(filtered[2].name, "main::start");
    }

    // ── normalize_debugger_output_line unit tests ──────────────────────────────

    #[test]
    pub(super) fn test_normalize_strips_single_db_prompt() {
        // Single prompt: "DB<1> main::(/path/file.pl:5):" -> "main::(/path/file.pl:5):"
        let result = DebugAdapter::normalize_debugger_output_line("DB<1> main::(/path/file.pl:5):");
        assert_eq!(result, "main::(/path/file.pl:5):");
    }

    #[test]
    pub(super) fn test_normalize_strips_multiple_db_prompts() {
        // Multiple prompts on one line — the while-let fix handles these.
        let result = DebugAdapter::normalize_debugger_output_line(
            "  DB<1>   DB<2> main::(/path/file.pl:5):",
        );
        assert_eq!(result, "main::(/path/file.pl:5):");
    }

    #[test]
    pub(super) fn test_normalize_strips_high_prompt_number() {
        // Prompt number > 99 — ensures '>' search is not length-limited.
        let result = DebugAdapter::normalize_debugger_output_line("DB<100> $x = 42");
        assert_eq!(result, "$x = 42");
    }

    #[test]
    pub(super) fn test_normalize_no_prompt_passthrough() {
        // Lines without a prompt must pass through unchanged (modulo trim).
        let result = DebugAdapter::normalize_debugger_output_line("  main::(/path/file.pl:5):");
        assert_eq!(result, "main::(/path/file.pl:5):");
    }

    #[test]
    pub(super) fn test_normalize_unclosed_prompt_passthrough() {
        // Malformed "DB<" without closing '>' must not loop forever and must not panic.
        let result = DebugAdapter::normalize_debugger_output_line("DB<incomplete");
        // The loop exits because find('>') returns None; the fragment remains.
        assert_eq!(result, "DB<incomplete");
    }

    #[test]
    pub(super) fn test_normalize_three_prompts_in_sequence() {
        // Three consecutive prompts — verifies loop handles arbitrary depth.
        let result = DebugAdapter::normalize_debugger_output_line("DB<1> DB<2> DB<3> my $x = 10;");
        assert_eq!(result, "my $x = 10;");
    }

    #[test]
    fn test_stack_trace_does_not_use_snapshot_in_degraded_path()
    -> Result<(), Box<dyn std::error::Error>> {
        // #933: When framed transport fails (degraded path), handle_stack_trace should return
        // empty frames instead of trying to parse the snapshot buffer.
        // The snapshot buffer contains the entire session history, so parsing it returns
        // the stale initial-stop frame first, not the current-stop frame.
        //
        // This test verifies the fix: degraded path returns Vec::new() which falls through
        // to session.stack_frames (populated by output reader).

        let adapter = DebugAdapter::new();
        adapter.seed_session_for_test();

        // Inject stale frames (simulating a previous stop's state)
        let stale_frame =
            StackFrame::new(1, "old_func".to_string(), Source::new("/old/file.pl"), 5);
        adapter.inject_stack_frames_for_test(vec![stale_frame]);

        // Simulate degraded transport: push old context line to snapshot buffer.
        // When framed transport fails, the else branch (lines 66-74 in frames.rs) is reached.
        // The snapshot buffer contains arrival-order history, so we push old + new in order.
        adapter.push_recent_output_line_for_test("main::(/test/file1.pl:4):"); // old context line
        adapter.push_recent_output_line_for_test("main::(/test/file2.pl:5):"); // current context line

        // Call handle_stack_trace with no framed output (simulating transport failure).
        // Note: we can't directly mock framed_output_lines=None without spawning a process,
        // so we rely on the absence of a live session stdin to trigger the else branch.
        // The adapter was seeded but with a mock session, so send_framed_debugger_commands
        // will fail or be skipped, reaching the degraded path.
        let response = adapter.handle_stack_trace(1, 1, None);

        // Extract the frames from response body
        match response {
            DapMessage::Response { body: Some(body), .. } => {
                if let Ok(trace_response) =
                    serde_json::from_value::<crate::protocol::StackTraceResponseBody>(body.clone())
                {
                    // FAILS if degraded path returns snapshot-parsed frames instead of empty.
                    // Before fix: frames.len() == 2 (old + new parsed from snapshot)
                    // After fix: frames.len() == 0 (empty from degraded path) OR len() == 1 (from session.stack_frames)
                    assert!(
                        trace_response.stack_frames.is_empty()
                            || trace_response.stack_frames.len() == 1,
                        "degraded path must return empty or fall through to session.stack_frames, not snapshot-parsed frames; got {} frames",
                        trace_response.stack_frames.len()
                    );
                }
            }
            _ => return Err("Expected response with body".into()),
        }
        Ok(())
    }
}
