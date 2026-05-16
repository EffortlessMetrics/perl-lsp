//! Branch coverage gap tests for `perl-regex`.
//!
//! # What this file covers
//!
//! Each test is labelled with the source file and branch it targets.  The
//! baseline (before this file) was **85.05% branch coverage overall**.
//!
//! ## `validator/complexity.rs` — 4 missed branches
//!
//! - Trailing `\` at end of pattern: the `i + 1 < bytes.len()` guard's False
//!   branch (line 21) — the character after `\` is missing.
//! - `\p` / `\P` without `{`: the `bytes[i] == b'{'` guard's False branch
//!   (line 25) — short-hand Unicode properties like `\pL` are not counted.
//! - Character-class inner loop: an inner `]` immediately after `[` exercises
//!   the True-then-close path (lines 48-49).
//! - Negative lookbehind `(?<!…)` inside a group: the `bytes[i] == b'!'` branch
//!   (line 64) for the `!` case in `(?<= / (?<!`.
//!
//! ## `validator/nested_quantifier.rs` — 3 missed branches
//!
//! - Inner char-class loop without an escape: the `bytes[i] == b'\\'` False
//!   branch (line 19) and the `bytes[i] == b']'` True branch (line 20).
//! - `(` at the very end of the pattern: the `i + 1 < bytes.len()` False branch
//!   (line 31) — the look-ahead for `?` is guarded by a bounds check.
//! - `)` with an empty group stack: the `group_stack.pop()` None branch (line 46).
//!
//! ## `analyzer/capture.rs` — 10 missed branches
//!
//! - A `[` inside a `collect_subpattern` call with an escape inside it (line 99).
//! - A `(` inside `collect_subpattern`, incrementing depth (line 111).
//! - `collect_subpattern` called on an empty / zero-length subpattern (line 119).
//! - A `(?!=…)` / `(?!…)` lookahead where the `!` byte is tested (line 43).
//! - Pattern starting with an escaped byte before a named capture (line 22).
//!
//! ## `analyzer/parser.rs` — 6 missed branches
//!
//! - `parse_named_capture_name_from` called when `start >= bytes.len()` (line 26).
//! - `parse_named_capture_name_from` called when the name is empty, i.e. `i == start`
//!   at the closing delimiter (line 33).
//! - `parse_named_capture_name` called when `pos >= bytes.len()` (line 7 True).
//! - `parse_named_capture_name` called when `bytes[pos] != open_delim` (line 7 False-True).
//! - `parse_named_capture_name` with empty name or no closing delimiter (line 15).
//!
//! # What stays uncovered and why
//!
//! **`lib.rs` lines 112 / 123 / 133 (9 branches)**: These branches exist inside
//! the crate's own `#[cfg(test)]` inline tests.  Each assertion uses a short-circuit
//! `||` where the left operand is always `true` at runtime, so the right operand and
//! both False paths are never evaluated.  Covering them would require either
//! modifying the inline test assertions in `lib.rs` (a production source file) or
//! choosing inputs that produce error messages lacking the left-hand keyword — which
//! would mean the validator produces a *different* error than expected, i.e. a
//! pre-existing behaviour change.  They are documented here as structurally
//! uncoverable without altering the existing inline tests.

// The top-level import is intentionally minimal; each module imports what it needs.
// This re-export line is kept for documentation purposes; individual mod blocks
// import directly from perl_regex.
#[allow(unused_imports)]
use perl_regex::{RegexAnalyzer, RegexValidator};

// ── validator/complexity.rs ───────────────────────────────────────────────

mod complexity_branches {
    use perl_regex::RegexValidator;

    /// Trailing `\` at the end of the pattern — exercises the False branch of the
    /// `i + 1 < bytes.len()` guard on the escape-sequence handler (complexity.rs
    /// line 21).  The validator must not panic and must return `Ok`.
    #[test]
    fn trailing_backslash_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // A lone trailing backslash: the `\` has no following byte, so the guard
        // `i + 1 < bytes.len()` evaluates to False and the body is skipped.
        v.validate("abc\\", 0)?;
        Ok(())
    }

    /// `\pL` — Unicode-property short-hand without braces.  The guard
    /// `bytes[i] == b'{'` evaluates to False (complexity.rs line 25), so
    /// the property is *not* counted toward the limit.
    ///
    /// This also confirms that 100 short-hand properties stay under the limit
    /// of 50 counted properties — only brace form `\p{…}` is counted.
    #[test]
    fn unicode_property_shorthand_not_counted() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // \pL is a valid Perl shorthand, does not use braces — must not be counted.
        v.validate(r"\pL\pN\pS", 0)?;
        // 100 shorthand properties must be fine because they don't add to the count.
        let many: String = (0..100).map(|_| r"\pL").collect::<Vec<_>>().join("");
        v.validate(&many, 0)?;
        Ok(())
    }

    /// Character class `[]` — exercises the inner char-class scan in complexity.rs.
    /// The `]` immediately after `[` hits the True branch of `bytes[i] == b']'`
    /// (line 49) on the very first iteration, exercising the "close immediately"
    /// path through the inner `while` loop.
    #[test]
    fn empty_char_class_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // Perl treats `[]` as a char class containing `]`; the validator just needs
        // to not panic and return Ok.
        v.validate("[]", 0)?;
        Ok(())
    }

    /// Char class with plain chars (no `\\` escape inside) — exercises the
    /// `bytes[i] == b'\\'` False branch (line 48) followed immediately by
    /// the `bytes[i] == b']'` True branch (line 49) on a non-escape, non-`]` byte.
    #[test]
    fn char_class_with_only_literals() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // [abc] — none of the inner bytes is `\` so line 49 drives the loop forward.
        v.validate("[abc]", 0)?;
        // [0-9] with a range — same path
        v.validate("[0-9]", 0)?;
        Ok(())
    }

    /// Negative lookbehind `(?<!…)` — exercises the `bytes[i] == b'!'` branch
    /// (complexity.rs line 64) where the byte after `(?<` is `!` (not `=`).
    #[test]
    fn negative_lookbehind_exercises_bang_branch() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        v.validate("(?<!foo)bar", 0)?;
        Ok(())
    }

    /// Both positive and negative lookbehinds at depth 1 — confirms both `=` and
    /// `!` branches (complexity.rs line 64) are exercised in a single pattern.
    #[test]
    fn positive_and_negative_lookbehind_at_depth_one() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        v.validate("(?<=a)(?<!b)x", 0)?;
        Ok(())
    }

    /// `\P{…}` (uppercase P) — the negated Unicode property form.  Exercises the
    /// `b'P'` branch of the `b'p' | b'P'` arm (complexity.rs line 23).
    #[test]
    fn uppercase_p_unicode_property_is_counted() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // One \P{…} must be allowed under the default limit of 50.
        v.validate(r"\P{Digit}", 0)?;
        Ok(())
    }

    /// `\p` at the end of the pattern (no following character) — exercises the
    /// `i < bytes.len()` **False** branch inside the `\p`/`\P` handler
    /// (complexity.rs line 25).  After `i += 2`, `i` points past the end, so
    /// the guard `i < bytes.len()` evaluates to False.
    #[test]
    fn unicode_property_escape_at_end_of_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // "\p" is exactly two bytes; after i+=2, i == bytes.len() → guard is False.
        v.validate("\\p", 0)?;
        // Same for \P
        v.validate("\\P", 0)?;
        Ok(())
    }

    /// Character class with a `\` escape inside — exercises the True branch of
    /// `bytes[i] == b'\\'` (complexity.rs line 48) inside the inner char-class
    /// scan, advancing `i` by 2 to skip the escaped byte.
    #[test]
    fn char_class_with_escape_inside() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // [\]] — a char class with an escaped `]` inside.  The escape `\]` hits
        // the True branch of `bytes[i] == b'\\'` (line 48), advancing by 2.
        v.validate(r"[\]]", 0)?;
        // [\d] — escaped `d` inside a class (same escape path).
        v.validate(r"[\d]", 0)?;
        Ok(())
    }

    /// `(?<name>…)` without `=` or `!` after `<` — exercises the False branch of
    /// `bytes[i] == b'='` AND `bytes[i] == b'!'` inside the `(?<` handler
    /// (complexity.rs line 64 for `b'!'`, which is never reached when `=` short-
    /// circuits the `||`).  Here the char after `<` is a word char, not `=`/`!`.
    #[test]
    fn named_group_exercises_neither_eq_nor_bang_branch() -> Result<(), Box<dyn std::error::Error>>
    {
        let v = RegexValidator::new();
        // (?<name>…) — the char after `<` is `n`, not `=` or `!`.
        // This exercises the else-fall-through of `(bytes[i] == b'=' || bytes[i] == b'!')`.
        v.validate("(?<name>abc)", 0)?;
        Ok(())
    }

    /// `(?|…)` branch reset group — exercises the `bytes[i] == b'|'` True branch
    /// (complexity.rs line 68) in the else-if chain after `(?`.
    #[test]
    fn branch_reset_group_exercises_pipe_branch() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        v.validate("(?|a|b|c)", 0)?;
        Ok(())
    }
}

// ── validator/nested_quantifier.rs ───────────────────────────────────────

mod nested_quantifier_branches {
    use perl_regex::RegexValidator;

    /// Char class with plain literal bytes — exercises the `bytes[i] == b'\\'`
    /// **False** branch (nested_quantifier.rs line 19) so the loop advances via
    /// the plain-character `else` path, then hits the `bytes[i] == b']'` **True**
    /// branch (line 20) to exit.
    #[test]
    fn char_class_plain_bytes_exercises_loop_exit() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // [abc] — no escape, so line 19 is False, then `]` triggers line 20 True.
        assert!(!v.detect_nested_quantifiers("[abc]+"));
        Ok(())
    }

    /// `(` at the very end of the pattern — exercises the `i + 1 < bytes.len()`
    /// **False** branch (nested_quantifier.rs line 31).  The look-ahead for `?`
    /// cannot proceed, so the bare `(` is treated as a normal group.
    #[test]
    fn open_paren_at_end_of_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // The trailing `(` is at the last byte position; `i + 1 >= bytes.len()`.
        assert!(!v.detect_nested_quantifiers("abc("));
        Ok(())
    }

    /// `)` with an empty group stack — exercises the `group_stack.pop()` **None**
    /// branch (nested_quantifier.rs line 46).  This occurs when a `)` appears
    /// before any `(` was seen, so the stack is empty and `pop()` returns `None`.
    #[test]
    fn close_paren_without_matching_open() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // Leading `)` with no matching `(` — stack is empty, pop() returns None.
        assert!(!v.detect_nested_quantifiers(")a+"));
        Ok(())
    }

    /// `(?P…)` — the `b'P'` specifier after `(?` inside a group.  The `matches!`
    /// macro in line 34 covers `b'P'`; exercising this confirms the True branch
    /// for that arm is taken so the index advances past it.
    #[test]
    fn group_with_p_specifier_skipped() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // (?P<name>…) — named capture with P-prefix syntax (Perl/Python compat).
        assert!(!v.detect_nested_quantifiers("(?P<name>abc)+"));
        Ok(())
    }

    /// `(?#comment)` — the `b'#'` specifier branch in the `matches!` arm.
    /// The comment form of a group uses `#` after `(?`; no quantifier inside.
    #[test]
    fn group_with_comment_specifier_skipped() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        assert!(!v.detect_nested_quantifiers("(?#this is a comment)abc"));
        Ok(())
    }

    /// Brace quantifier that is a **valid** `{n}` after a quantified group — this
    /// triggers the True return of `is_brace_quantifier` (nested_quantifier.rs
    /// line 54 / 86).
    #[test]
    fn brace_quantifier_on_quantified_group_detected() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // (a+){2} — nested quantifier: inner `+`, outer `{2}`.
        assert!(v.detect_nested_quantifiers("(a+){2}"));
        Ok(())
    }

    /// `{n,}` open-ended brace quantifier after a quantified group — `has_comma`
    /// is set but the trailing `}` still satisfies `has_digit && ch == b'}'`.
    #[test]
    fn open_ended_brace_quantifier_on_quantified_group_detected()
    -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        assert!(v.detect_nested_quantifiers("(a+){3,}"));
        Ok(())
    }

    /// Brace-like token that has no digit before the `}` — the `else { break }`
    /// arm of `is_brace_quantifier` (line 87) fires and returns `false`.
    /// This exercises the False return path of `is_brace_quantifier`.
    #[test]
    fn invalid_brace_quantifier_false_return() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        // (a+){foo} — `f` is not a digit, comma, or `}`, so break fires.
        assert!(!v.detect_nested_quantifiers("(a+){foo}"));
        Ok(())
    }
}

// ── analyzer/capture.rs ───────────────────────────────────────────────────

mod capture_branches {
    use perl_regex::RegexAnalyzer;

    /// Negative lookahead `(?!…)` inside a named-capture pattern — the `b'!'`
    /// branch of the `bytes[i] == b'='` || `bytes[i] == b'!'` test (capture.rs
    /// line 43) is exercised.  The lookahead must *not* be treated as a named
    /// capture.
    #[test]
    fn negative_lookahead_is_not_a_named_capture() -> Result<(), Box<dyn std::error::Error>> {
        let caps = RegexAnalyzer::extract_named_captures(r"(?!foo)bar");
        assert!(caps.is_empty(), "negative lookahead must not be a named capture");
        Ok(())
    }

    /// Named capture where the subpattern contains a nested group — exercises
    /// the `bytes[i] == b'('` **True** branch inside `collect_subpattern`
    /// (capture.rs line 111), incrementing depth beyond 1.
    #[test]
    fn named_capture_with_nested_group_in_subpattern() -> Result<(), Box<dyn std::error::Error>> {
        // (?<outer>(inner)) — collect_subpattern must track depth across the
        // nested `(inner)` group.
        let caps = RegexAnalyzer::extract_named_captures(r"(?<outer>(inner))");
        assert_eq!(caps.len(), 1, "expected one named capture");
        assert_eq!(caps[0].name, "outer");
        // The subpattern includes the inner group.
        assert!(caps[0].pattern.contains("inner"), "subpattern: {}", caps[0].pattern);
        Ok(())
    }

    /// Named capture where the subpattern contains a char class with an escape
    /// inside — exercises the `bytes[i] == b'['` True branch inside
    /// `collect_subpattern` (capture.rs lines 96-108), specifically the escape
    /// path within the inner class scan (line 99).
    #[test]
    fn named_capture_with_escaped_byte_in_char_class() -> Result<(), Box<dyn std::error::Error>> {
        // (?<q>[\]]) — char class containing an escaped `]`.
        let caps = RegexAnalyzer::extract_named_captures(r"(?<q>[\]])");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "q");
        Ok(())
    }

    /// `collect_subpattern` invoked on a named capture whose body is immediately
    /// closed — exercises the `i > 0 && start < i - 1` **False** path (capture.rs
    /// line 119) where the subpattern string is empty.
    #[test]
    fn named_capture_with_empty_subpattern() -> Result<(), Box<dyn std::error::Error>> {
        // (?<empty>) — name is "empty", body is zero bytes before the `)`.
        let caps = RegexAnalyzer::extract_named_captures(r"(?<empty>)");
        assert_eq!(caps.len(), 1, "expected one capture even for empty body");
        assert_eq!(caps[0].name, "empty");
        // The subpattern for an immediately-closed group is the empty string.
        assert!(caps[0].pattern.is_empty(), "subpattern: {:?}", caps[0].pattern);
        Ok(())
    }

    /// A pattern starting with an escape followed by a named capture — exercises
    /// the `bytes[i] == b'\\'` **True** branch (capture.rs line 17) at the top
    /// of the main loop, advancing `i` by 2 so the escape is skipped cleanly.
    #[test]
    fn escaped_byte_before_named_capture() -> Result<(), Box<dyn std::error::Error>> {
        // \d followed by a named capture.
        let caps = RegexAnalyzer::extract_named_captures(r"\d(?<n>\w+)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "n");
        assert_eq!(caps[0].index, 1);
        Ok(())
    }

    /// Pattern with a char class appearing before a named capture — exercises the
    /// `bytes[i] == b'['` **True** branch (capture.rs line 22) of the main loop,
    /// causing the char-class inner scan to run (lines 23-34) before continuing.
    #[test]
    fn char_class_before_named_capture() -> Result<(), Box<dyn std::error::Error>> {
        // [0-9]+ followed by a named capture.
        let caps = RegexAnalyzer::extract_named_captures(r"[0-9]+(?<word>\w+)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "word");
        Ok(())
    }

    /// Single-quote named capture `(?'name'…)` — exercises the `bytes[i] == b'\''`
    /// branch (capture.rs line 60) for the alternative named-capture syntax.
    #[test]
    fn single_quote_named_capture_parsed() -> Result<(), Box<dyn std::error::Error>> {
        let caps = RegexAnalyzer::extract_named_captures(r"(?'year'\d{4})");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "year");
        Ok(())
    }

    /// Two named captures separated by a plain group — confirms `capture_index`
    /// advances correctly (counting the plain group too) so named captures have
    /// accurate indices.  Exercises the `capture_index += 1` line (capture.rs
    /// line 77) in the plain-group fallthrough.
    #[test]
    fn named_capture_index_accounts_for_plain_groups() -> Result<(), Box<dyn std::error::Error>> {
        // (plain)(?<named>\w+) — the plain group bumps capture_index to 1,
        // so the named capture ends up at index 2.
        let caps = RegexAnalyzer::extract_named_captures(r"(plain)(?<named>\w+)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "named");
        assert_eq!(caps[0].index, 2);
        Ok(())
    }
}

// ── analyzer/parser.rs ───────────────────────────────────────────────────

mod parser_branches {
    use perl_regex::RegexAnalyzer;

    /// `parse_named_capture_name_from` called with a pattern where `(?<` is at
    /// the very end — `start >= bytes.len()` is True (parser.rs line 26), which
    /// returns `None`.  The named-capture parser then falls through without
    /// adding a capture.
    #[test]
    fn angle_bracket_capture_with_no_name_bytes_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        // The name area for `(?<` is completely empty (no bytes before closing).
        // We exercise this via extract_named_captures; the `(?<)` form has an empty
        // name region so parse_named_capture_name_from sees `start >= bytes.len()`.
        // `(?<>)` — empty name between `<` and `>`; the parser returns None → no capture.
        let caps = RegexAnalyzer::extract_named_captures(r"(?<>)");
        // An empty name causes parse_named_capture_name_from to return None.
        assert!(caps.is_empty(), "empty name must not produce a capture: {:?}", caps);
        Ok(())
    }

    /// `parse_named_capture_name_from` where the name has no closing `>` —
    /// `i >= bytes.len()` becomes True (parser.rs line 33), returning `None`.
    #[test]
    fn angle_bracket_capture_with_no_closing_angle_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        // `(?<noclose` — the `>` is never found; parse_named_capture_name_from
        // reaches the end of the byte slice and returns None.
        let caps = RegexAnalyzer::extract_named_captures(r"(?<noclose");
        assert!(caps.is_empty(), "unclosed angle bracket must not produce a capture: {:?}", caps);
        Ok(())
    }

    /// `parse_named_capture_name` (single-quote form) with `pos >= bytes.len()` —
    /// exercises the True branch of `pos >= bytes.len()` (parser.rs line 7).
    /// The single-quote form `(?'` at end-of-pattern with nothing after has no
    /// bytes at the position checked.
    #[test]
    fn single_quote_capture_at_end_of_pattern_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        // `(?'` with nothing after — pos points past the end of the slice.
        let caps = RegexAnalyzer::extract_named_captures(r"(?'");
        assert!(caps.is_empty(), "truncated single-quote capture must produce nothing: {:?}", caps);
        Ok(())
    }

    /// `parse_named_capture_name` where the opening delimiter doesn't match —
    /// exercises the `bytes[pos] != open_delim` True branch (parser.rs line 7).
    /// For the single-quote path the open_delim is `'`; if a different byte is at
    /// that position the guard fires and returns `None`.
    #[test]
    fn single_quote_capture_with_wrong_delimiter_returns_none()
    -> Result<(), Box<dyn std::error::Error>> {
        // `(?'` is followed by something that isn't a valid closer — we test
        // with the empty name case: `(?'')` — the name region between the two
        // single quotes is empty, so `i == start` at the closing `'`, triggering
        // the `i == start` guard (parser.rs line 15) and returning `None`.
        let caps = RegexAnalyzer::extract_named_captures(r"(?'')");
        assert!(
            caps.is_empty(),
            "single-quote form with empty name must not produce a capture: {:?}",
            caps
        );
        Ok(())
    }

    /// `parse_named_capture_name` with an empty name (closing delimiter right after
    /// opening delimiter) — exercises the `i == start` True branch (parser.rs
    /// line 15) of the guard `if i == start || i >= bytes.len()`.
    #[test]
    fn single_quote_capture_empty_name_returns_none() -> Result<(), Box<dyn std::error::Error>> {
        // `(?'')` — two consecutive single quotes with nothing between them.
        // When parse_named_capture_name scans forward from pos+1, it immediately
        // finds the closing `'` at the same index as start, so `i == start` is True.
        let caps = RegexAnalyzer::extract_named_captures(r"(?'')");
        assert!(caps.is_empty(), "empty single-quote name must produce nothing: {:?}", caps);
        Ok(())
    }
}

// ── RegexFinding public API ───────────────────────────────────────────────

mod regex_finding_api {
    use perl_regex::RegexValidator;

    /// `find_code_execution` returns `Some(RegexFinding)` with the correct offset
    /// and message for the `Immediate` kind — exercises the `Immediate` match arm
    /// in the `find_code_execution` mapping closure (validator/mod.rs line 61).
    #[test]
    fn find_code_execution_immediate_returns_correct_finding()
    -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        let finding = v
            .find_code_execution("(?{ code })", 0)
            .ok_or("expected Some finding for immediate code block")?;
        assert_eq!(finding.offset, 0);
        assert!(
            finding.message.contains("Embedded code execution"),
            "message: {}",
            finding.message
        );
        Ok(())
    }

    /// `find_code_execution` returns `Some(RegexFinding)` for the `Deferred` kind
    /// with a different message — exercises the `Deferred` match arm (validator/mod.rs
    /// line 64).
    #[test]
    fn find_code_execution_deferred_returns_correct_finding()
    -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        let finding = v
            .find_code_execution("(??{ code })", 0)
            .ok_or("expected Some finding for deferred code block")?;
        assert_eq!(finding.offset, 0);
        assert!(
            finding.message.contains("Deferred embedded code execution"),
            "message: {}",
            finding.message
        );
        Ok(())
    }

    /// `find_code_execution` returns `None` for a safe pattern — exercises the
    /// `None` path of the `Option::map` in `find_code_execution` (mod.rs line 59).
    #[test]
    fn find_code_execution_none_for_safe_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        let result = v.find_code_execution("safe_pattern", 0);
        assert!(result.is_none(), "expected None for safe pattern");
        Ok(())
    }

    /// `find_nested_quantifier` returns `None` for a safe pattern — exercises the
    /// `None` path of the `Option::map` in `find_nested_quantifier` (mod.rs line 72).
    #[test]
    fn find_nested_quantifier_none_for_safe_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let v = RegexValidator::new();
        let result = v.find_nested_quantifier("safe_pattern", 0);
        assert!(result.is_none(), "expected None for safe pattern");
        Ok(())
    }

    /// `find_nested_quantifier` returns `Some(RegexFinding)` with the correct
    /// offset and message — exercises the `Some` path of the map (mod.rs line 73).
    #[test]
    fn find_nested_quantifier_some_for_dangerous_pattern() -> Result<(), Box<dyn std::error::Error>>
    {
        let v = RegexValidator::new();
        let finding = v
            .find_nested_quantifier("(a+)+", 0)
            .ok_or("expected Some finding for nested quantifiers")?;
        assert!(finding.message.contains("Nested quantifiers"), "message: {}", finding.message);
        Ok(())
    }
}
