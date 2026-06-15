/// Returns `true` when a source file should be skipped wholesale by the print-macro check.
///
/// Files with a file-level `#![allow(clippy::print_stderr)]` or
/// `#![allow(clippy::print_stdout)]` attribute have been explicitly opted out of the
/// rule (e.g. `cli.rs` in the LSP binary crate). The attribute must appear in the
/// first 30 lines of the file (the module-doc / crate-doc block).
pub(super) fn file_has_print_allow(lines: &[String]) -> bool {
    lines.iter().take(30).any(|line| line_has_inner_print_allow_attr(line))
}

fn line_has_inner_print_allow_attr(line: &str) -> bool {
    line.contains("#![allow(")
        && (line.contains("clippy::print_stderr")
            || line.contains("clippy::print_stdout")
            || line.contains("clippy::print_"))
}

pub(super) fn line_has_outer_print_allow_attr(line: &str) -> bool {
    line.contains("#[allow(")
        && (line.contains("clippy::print_stderr")
            || line.contains("clippy::print_stdout")
            || line.contains("clippy::print_"))
}

pub(super) fn line_is_whole_line_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

#[derive(Default)]
pub(super) struct PrintAllowScope {
    pending_attr: bool,
    active_brace_depth: usize,
}

impl PrintAllowScope {
    pub(super) fn note_attribute(&mut self) {
        self.pending_attr = true;
    }

    pub(super) fn allows_current_line(&self) -> bool {
        self.pending_attr || self.active_brace_depth > 0
    }

    pub(super) fn observe_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || line_is_whole_line_comment(line) || trimmed.starts_with("#[") {
            return;
        }

        if self.active_brace_depth > 0 {
            self.apply_brace_delta(line);
            return;
        }

        if self.pending_attr {
            self.pending_attr = false;
            let delta = brace_delta(line);
            if delta > 0 {
                self.active_brace_depth = delta as usize;
            }
        }
    }

    fn apply_brace_delta(&mut self, line: &str) {
        let delta = brace_delta(line);
        if delta.is_negative() {
            self.active_brace_depth = self.active_brace_depth.saturating_sub(delta.unsigned_abs());
        } else {
            self.active_brace_depth = self.active_brace_depth.saturating_add(delta as usize);
        }
    }
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|ch| *ch == '{').count() as isize;
    let closes = line.chars().filter(|ch| *ch == '}').count() as isize;
    opens - closes
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── allows_current_line discriminators ───────────────────────────────────

    #[test]
    fn allows_current_line_pending_attr_only_is_true() {
        let mut scope = PrintAllowScope::default();
        scope.note_attribute();
        // pending_attr=true, active_brace_depth=0 → true
        assert!(scope.allows_current_line());
    }

    #[test]
    fn allows_current_line_active_brace_depth_only_is_true() {
        let mut scope = PrintAllowScope::default();
        // Simulate entering a brace scope: note_attribute then observe a
        // line that opens a brace.  After observe_line, pending_attr is
        // consumed and active_brace_depth becomes 1.
        scope.note_attribute();
        scope.observe_line("pub fn foo() {");
        // pending_attr=false, active_brace_depth=1 → allows_current_line=true
        assert!(!scope.pending_attr, "pending_attr should have been consumed");
        assert_eq!(scope.active_brace_depth, 1, "brace should be open");
        assert!(scope.allows_current_line(), "active_brace_depth > 0 → true");
    }

    #[test]
    fn allows_current_line_neither_flag_is_false() {
        let scope = PrintAllowScope::default();
        // pending_attr=false, active_brace_depth=0 → false
        assert!(!scope.allows_current_line());
    }

    // ── observe_line: active_brace_depth branch ──────────────────────────────

    #[test]
    fn observe_line_while_in_brace_scope_applies_delta() {
        let mut scope = PrintAllowScope::default();
        // Enter brace scope via pending_attr path.
        scope.note_attribute();
        scope.observe_line("fn foo() {");     // depth becomes 1
        assert_eq!(scope.active_brace_depth, 1);
        // Observe a line that adds one more open brace.
        scope.observe_line("if true {");      // depth becomes 2
        assert_eq!(scope.active_brace_depth, 2);
        // Observe a closing brace — depth decrements.
        scope.observe_line("}");              // depth becomes 1
        assert_eq!(scope.active_brace_depth, 1);
    }

    #[test]
    fn observe_line_pending_attr_consumed_and_depth_set() {
        let mut scope = PrintAllowScope::default();
        scope.note_attribute();
        assert!(scope.pending_attr);
        scope.observe_line("fn foo() {");
        // pending_attr cleared, active_brace_depth opened
        assert!(!scope.pending_attr);
        assert_eq!(scope.active_brace_depth, 1);
    }

    #[test]
    fn observe_line_skips_empty_comment_and_attr_lines() {
        let mut scope = PrintAllowScope::default();
        scope.note_attribute();
        // These lines must not consume pending_attr
        scope.observe_line("   ");            // whitespace-only
        scope.observe_line("// comment");    // whole-line comment
        scope.observe_line("#[inline]");     // attribute line
        assert!(scope.pending_attr, "pending_attr should survive non-consuming lines");
    }

    // ── apply_brace_delta negative delta ─────────────────────────────────────

    #[test]
    fn apply_brace_delta_decrements_depth_on_close() {
        let mut scope = PrintAllowScope::default();
        scope.note_attribute();
        scope.observe_line("fn foo() {");    // pending_attr consumed; depth = 1
        assert_eq!(scope.active_brace_depth, 1);
        scope.observe_line("}");             // apply_brace_delta: depth → 0
        assert_eq!(scope.active_brace_depth, 0);
    }

    #[test]
    fn apply_brace_delta_saturates_at_zero_on_excess_closes() {
        let mut scope = PrintAllowScope::default();
        scope.note_attribute();
        // Open two braces in a single line.
        scope.observe_line("fn foo() { if true {");   // depth = 2
        assert_eq!(scope.active_brace_depth, 2);
        // Close three braces — more than the depth; should saturate at 0.
        scope.observe_line("} } }");                   // depth → 0, not underflow
        assert_eq!(scope.active_brace_depth, 0);
    }

    // ── brace_delta edge cases ────────────────────────────────────────────────

    #[test]
    fn brace_delta_balanced_line_returns_zero() {
        assert_eq!(brace_delta("let x = {1};"), 0);
    }

    #[test]
    fn brace_delta_open_only() {
        assert_eq!(brace_delta("fn foo() {"), 1);
    }

    #[test]
    fn brace_delta_close_only() {
        assert_eq!(brace_delta("}"), -1);
    }

    // ── file-level allow discriminators ──────────────────────────────────────

    #[test]
    fn file_has_print_allow_detects_inner_print_stderr() {
        let lines: Vec<String> =
            vec!["#![allow(clippy::print_stderr)]".to_owned()];
        assert!(file_has_print_allow(&lines));
    }

    #[test]
    fn file_has_print_allow_detects_inner_print_stdout() {
        let lines: Vec<String> =
            vec!["#![allow(clippy::print_stdout)]".to_owned()];
        assert!(file_has_print_allow(&lines));
    }

    #[test]
    fn file_has_print_allow_only_checks_first_30_lines() {
        let mut lines: Vec<String> = (0..30).map(|_| "// plain comment".to_owned()).collect();
        lines.push("#![allow(clippy::print_stderr)]".to_owned()); // line 31
        assert!(!file_has_print_allow(&lines));
    }

    #[test]
    fn line_has_outer_print_allow_attr_detects_outer_bracket() {
        assert!(line_has_outer_print_allow_attr(
            "#[allow(clippy::print_stderr, clippy::print_stdout)]"
        ));
    }

    #[test]
    fn line_has_outer_print_allow_attr_rejects_inner_bracket() {
        assert!(!line_has_outer_print_allow_attr(
            "#![allow(clippy::print_stderr)]"
        ));
    }

    #[test]
    fn line_is_whole_line_comment_true_for_double_slash() {
        assert!(line_is_whole_line_comment("  // this is a comment"));
    }

    #[test]
    fn line_is_whole_line_comment_false_for_code() {
        assert!(!line_is_whole_line_comment("let x = 1; // inline comment"));
    }
}
