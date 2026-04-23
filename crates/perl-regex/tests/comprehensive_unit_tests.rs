//! Comprehensive unit tests for the perl-regex crate.
//!
//! Covers: RegexError, RegexValidator (validate, detects_code_execution,
//! detect_nested_quantifiers), Default impl, and edge cases.

use perl_regex::{RegexError, RegexValidator};

// ── RegexError ──────────────────────────────────────────────────────────

#[test]
fn regex_error_syntax_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let err = RegexError::syntax("bad pattern", 42);
    match &err {
        RegexError::Syntax { message, offset } => {
            assert_eq!(message, "bad pattern");
            assert_eq!(*offset, 42);
        }
    }
    Ok(())
}

#[test]
fn regex_error_display_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = RegexError::syntax("unmatched paren", 7);
    let display = format!("{err}");
    assert_eq!(display, "unmatched paren at offset 7");
    Ok(())
}

#[test]
fn regex_error_clone_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let a = RegexError::syntax("x", 0);
    let b = a.clone();
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn regex_error_debug_contains_variant() -> Result<(), Box<dyn std::error::Error>> {
    let err = RegexError::syntax("msg", 1);
    let dbg = format!("{err:?}");
    assert!(dbg.contains("Syntax"));
    Ok(())
}

#[test]
fn regex_error_syntax_zero_offset() -> Result<(), Box<dyn std::error::Error>> {
    let err = RegexError::syntax("at start", 0);
    assert_eq!(format!("{err}"), "at start at offset 0");
    Ok(())
}

#[test]
fn regex_error_syntax_large_offset() -> Result<(), Box<dyn std::error::Error>> {
    let err = RegexError::syntax("far away", usize::MAX);
    match &err {
        RegexError::Syntax { offset, .. } => assert_eq!(*offset, usize::MAX),
    }
    Ok(())
}

#[test]
fn regex_error_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    let err = RegexError::syntax("", 5);
    assert_eq!(format!("{err}"), " at offset 5");
    Ok(())
}

#[test]
fn regex_error_accepts_string_owned() -> Result<(), Box<dyn std::error::Error>> {
    let msg = String::from("owned message");
    let err = RegexError::syntax(msg, 10);
    match &err {
        RegexError::Syntax { message, .. } => assert_eq!(message, "owned message"),
    }
    Ok(())
}

// ── RegexValidator construction ─────────────────────────────────────────

#[test]
fn validator_new_returns_instance() -> Result<(), Box<dyn std::error::Error>> {
    let _v = RegexValidator::new();
    Ok(())
}

#[test]
fn validator_default_matches_new() -> Result<(), Box<dyn std::error::Error>> {
    // Both should behave identically on the same input
    let v1 = RegexValidator::new();
    let v2 = RegexValidator::default();
    let pattern = "(a+)+";
    let r1 = v1.validate(pattern, 0);
    let r2 = v2.validate(pattern, 0);
    assert_eq!(r1.is_err(), r2.is_err());
    Ok(())
}

// ── validate() — safe patterns ──────────────────────────────────────────

#[test]
fn validate_empty_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("", 0)?;
    Ok(())
}

#[test]
fn validate_simple_literal() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("hello", 0)?;
    Ok(())
}

#[test]
fn validate_single_quantifier() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("a+", 0)?;
    v.validate("b*", 0)?;
    v.validate("c?", 0)?;
    Ok(())
}

#[test]
fn validate_character_class() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("[a-z]+", 0)?;
    v.validate("[^0-9]", 0)?;
    Ok(())
}

#[test]
fn validate_alternation() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("foo|bar|baz", 0)?;
    Ok(())
}

#[test]
fn validate_anchors() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("^start$", 0)?;
    v.validate(r"\bhello\b", 0)?;
    Ok(())
}

#[test]
fn validate_non_capturing_group_without_outer_quantifier() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    // Non-capturing group without an outer quantifier is fine
    v.validate("(?:abc)", 0)?;
    Ok(())
}

#[test]
fn validate_non_capturing_group_with_outer_quantifier_ok() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    // Non-capturing group with an outer quantifier but no inner quantifier is safe
    v.validate("(?:abc)+", 0)?;
    Ok(())
}

#[test]
fn validate_named_capture() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(?<name>\\w+)", 0)?;
    Ok(())
}

#[test]
fn validate_lookahead() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("foo(?=bar)", 0)?;
    v.validate("foo(?!bar)", 0)?;
    Ok(())
}

#[test]
fn validate_lookbehind_simple() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(?<=foo)bar", 0)?;
    v.validate("(?<!foo)bar", 0)?;
    Ok(())
}

#[test]
fn validate_escaped_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate(r"\(\)\[\]\{\}\+\*\?", 0)?;
    Ok(())
}

#[test]
fn validate_unicode_literal_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("café", 0)?;
    v.validate("日本語", 0)?;
    v.validate("🦀+", 0)?;
    Ok(())
}

#[test]
fn validate_single_unicode_property() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate(r"\p{Latin}", 0)?;
    v.validate(r"\P{Digit}", 0)?;
    Ok(())
}

#[test]
fn validate_nested_quantifiers_no_longer_hard_error() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // validate() no longer rejects nested quantifiers (they are advisory, not fatal)
    v.validate("(a+)+", 100)?;
    // detect_nested_quantifiers() still detects them for callers that want to warn
    assert!(v.detect_nested_quantifiers("(a+)+"));
    Ok(())
}

#[test]
fn validate_group_without_quantifier_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(abc)(def)", 0)?;
    Ok(())
}

#[test]
fn validate_quantifier_on_group_without_inner_quantifier() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    v.validate("(abc)+", 0)?;
    v.validate("(abc)*", 0)?;
    v.validate("(abc)?", 0)?;
    Ok(())
}

// ── validate() — nested quantifiers are now accepted (advisory only) ──

#[test]
fn validate_accepts_nested_plus() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // validate() no longer rejects nested quantifiers
    v.validate("(a+)+", 0)?;
    // but detect_nested_quantifiers() still flags them
    assert!(v.detect_nested_quantifiers("(a+)+"));
    Ok(())
}

#[test]
fn validate_accepts_nested_star() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(a*)*", 0)?;
    assert!(v.detect_nested_quantifiers("(a*)*"));
    Ok(())
}

#[test]
fn validate_accepts_star_on_plus_group() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(a+)*", 0)?;
    assert!(v.detect_nested_quantifiers("(a+)*"));
    Ok(())
}

#[test]
fn validate_accepts_plus_on_star_group() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(a*)+", 0)?;
    assert!(v.detect_nested_quantifiers("(a*)+"));
    Ok(())
}

#[test]
fn validate_accepts_question_on_plus_group() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(a+)?", 0)?;
    assert!(v.detect_nested_quantifiers("(a+)?"));
    Ok(())
}

#[test]
fn validate_accepts_brace_quantifier_on_quantified_group() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    v.validate("(a+){2,5}", 0)?;
    assert!(v.detect_nested_quantifiers("(a+){2,5}"));
    Ok(())
}

// ── detect_nested_quantifiers() direct tests ────────────────────────────

#[test]
fn nested_quantifiers_false_for_empty() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detect_nested_quantifiers(""));
    Ok(())
}

#[test]
fn nested_quantifiers_false_for_simple() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detect_nested_quantifiers("abc"));
    assert!(!v.detect_nested_quantifiers("a+"));
    assert!(!v.detect_nested_quantifiers("[a-z]+"));
    Ok(())
}

#[test]
fn nested_quantifiers_true_classic_cases() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(v.detect_nested_quantifiers("(a+)+"));
    assert!(v.detect_nested_quantifiers("(a*)*"));
    assert!(v.detect_nested_quantifiers("(a?)*"));
    Ok(())
}

#[test]
fn nested_quantifiers_escaped_paren_not_group() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Escaped parens are literal, not groups
    assert!(!v.detect_nested_quantifiers(r"\(a+\)+"));
    Ok(())
}

#[test]
fn nested_quantifiers_non_capturing_group() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(v.detect_nested_quantifiers("(?:a+)+"));
    Ok(())
}

#[test]
fn nested_quantifiers_multiple_groups_only_last_nested() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // first group is safe, second has nested quantifiers
    assert!(v.detect_nested_quantifiers("(abc)(a+)+"));
    Ok(())
}

#[test]
fn nested_quantifiers_group_without_outer_quantifier() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // inner quantifier but no outer quantifier on the group
    assert!(!v.detect_nested_quantifiers("(a+)b"));
    Ok(())
}

#[test]
fn nested_quantifiers_literal_brace_after_group_is_not_detected()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Literal brace after a grouped quantified expression should not be treated as {n}
    assert!(!v.detect_nested_quantifiers("(a+){foo}"));
    Ok(())
}

#[test]
fn nested_quantifiers_invalid_brace_quantifier_is_not_detected()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Missing closing brace is not a valid quantifier marker.
    assert!(!v.detect_nested_quantifiers("(a+){2,5"));
    Ok(())
}

#[test]
fn nested_quantifiers_open_ended_brace_is_detected()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Open-ended {n,} is a valid Perl quantifier — must still flag nested use.
    assert!(v.detect_nested_quantifiers("(a+){2,}"));
    Ok(())
}

// ── detects_code_execution() ────────────────────────────────────────────

#[test]
fn code_execution_empty_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detects_code_execution(""));
    Ok(())
}

#[test]
fn code_execution_safe_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detects_code_execution("hello"));
    assert!(!v.detects_code_execution("(abc)"));
    assert!(!v.detects_code_execution("(?:abc)"));
    assert!(!v.detects_code_execution("(?=abc)"));
    assert!(!v.detects_code_execution("(?!abc)"));
    assert!(!v.detects_code_execution("(?<=abc)"));
    assert!(!v.detects_code_execution("(?<!abc)"));
    Ok(())
}

#[test]
fn code_execution_detects_eval_block() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(v.detects_code_execution("(?{ print 'hi' })"));
    Ok(())
}

#[test]
fn code_execution_detects_deferred_eval() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(v.detects_code_execution("(??{ $code })"));
    Ok(())
}

#[test]
fn code_execution_detects_embedded_in_larger_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(v.detects_code_execution("abc(?{ die })def"));
    assert!(v.detects_code_execution("^(\\w+)(??{ gen_re() })$"));
    Ok(())
}

#[test]
fn code_execution_escaped_paren_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Escaped open paren: literal, not a group start
    assert!(!v.detects_code_execution(r"\(?{ code }"));
    Ok(())
}

#[test]
fn code_execution_brace_without_question_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detects_code_execution("({stuff})"));
    Ok(())
}

#[test]
fn code_execution_question_without_brace_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detects_code_execution("(?:abc)"));
    assert!(!v.detects_code_execution("(?=abc)"));
    Ok(())
}

#[test]
fn code_execution_double_question_without_brace() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?? followed by something other than { is not code execution
    assert!(!v.detects_code_execution("(??x)"));
    Ok(())
}

#[test]
fn code_execution_markers_inside_char_class_are_safe() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    assert!(!v.detects_code_execution(r"[(?{]"));
    assert!(!v.detects_code_execution(r"[abc(??{xyz}]"));
    Ok(())
}

#[test]
fn code_execution_after_char_class_is_detected() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // A clean char class followed by a real (?{ }) — only the code after the class is real.
    // This is the discriminating case: pre-fix code would short-circuit on [a-z] contents
    // spuriously; post-fix code skips the class and detects the real (?{ run() }).
    assert!(v.detects_code_execution(r"[a-z](?{ run() })"));
    // Original case: class content looks like a code marker but real (?{ }) follows it.
    assert!(v.detects_code_execution(r"[(?{a-z}](?{ run() })"));
    Ok(())
}

#[test]
fn code_execution_multiple_code_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Returns true on first one found
    assert!(v.detects_code_execution("(?{ a })(?{ b })"));
    Ok(())
}

// ── Unicode property limits ─────────────────────────────────────────────

#[test]
fn validate_under_unicode_property_limit() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // 50 properties should be exactly at the limit
    let pattern: String = (0..50).map(|_| r"\p{L}").collect::<Vec<_>>().join("");
    v.validate(&pattern, 0)?;
    Ok(())
}

#[test]
fn validate_exceeds_unicode_property_limit() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // 51 properties should exceed the limit of 50
    let pattern: String = (0..51).map(|_| r"\p{L}").collect::<Vec<_>>().join("");
    let result = v.validate(&pattern, 0);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("Too many Unicode properties"));
    Ok(())
}

#[test]
fn validate_uppercase_p_counted_same() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // \P{...} should also count toward the limit
    let pattern: String = (0..51).map(|_| r"\P{Digit}").collect::<Vec<_>>().join("");
    let result = v.validate(&pattern, 0);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn validate_unicode_p_without_brace_not_counted() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // \pL (shorthand without braces) should not count
    let pattern: String = (0..100).map(|_| r"\pL").collect::<Vec<_>>().join("");
    v.validate(&pattern, 0)?;
    Ok(())
}

// ── Lookbehind nesting limits ───────────────────────────────────────────

#[test]
fn validate_shallow_lookbehind_nesting() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Single lookbehind is fine
    v.validate("(?<=a)b", 0)?;
    Ok(())
}

#[test]
fn validate_deep_lookbehind_nesting_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Build 11 nested lookbehinds (exceeds max_nesting=10)
    let mut pattern = String::from("x");
    for _ in 0..11 {
        pattern = format!("(?<={})", pattern);
    }
    let result = v.validate(&pattern, 0);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("lookbehind nesting too deep"));
    Ok(())
}

#[test]
fn validate_exactly_max_lookbehind_nesting_ok() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Build exactly 10 nested lookbehinds (at limit, not over)
    let mut pattern = String::from("x");
    for _ in 0..10 {
        pattern = format!("(?<={})", pattern);
    }
    v.validate(&pattern, 0)?;
    Ok(())
}

// ── Branch reset nesting limits ─────────────────────────────────────────

#[test]
fn validate_single_branch_reset() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(?|a|b|c)", 0)?;
    Ok(())
}

#[test]
fn validate_deep_branch_reset_nesting_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Build 11 nested branch resets (exceeds max_nesting=10)
    let mut pattern = String::from("x");
    for _ in 0..11 {
        pattern = format!("(?|{})", pattern);
    }
    let result = v.validate(&pattern, 0);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("branch reset nesting too deep"));
    Ok(())
}

#[test]
fn validate_branch_reset_too_many_branches() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // 51 branches in a branch reset (exceeds max of 50)
    let branches: String = (0..51).map(|i| format!("a{i}")).collect::<Vec<_>>().join("|");
    let pattern = format!("(?|{branches})");
    let result = v.validate(&pattern, 0);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Too many branches"));
    Ok(())
}

#[test]
fn validate_branch_reset_at_limit_ok() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Exactly 50 branches (at limit), the group starts with one branch and | adds more
    // So 50 pipes = 51 branches total. We need 49 pipes for 50 branches.
    let branches: String = (0..50).map(|i| format!("a{i}")).collect::<Vec<_>>().join("|");
    let pattern = format!("(?|{branches})");
    v.validate(&pattern, 0)?;
    Ok(())
}

// ── Character class handling ────────────────────────────────────────────

#[test]
fn validate_character_class_with_escaped_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate(r"[\]]", 0)?;
    Ok(())
}

#[test]
fn validate_character_class_with_special_chars() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate(r"[+*?{}()]", 0)?;
    Ok(())
}

// ── Misc edge cases ────────────────────────────────────────────────────

#[test]
fn validate_only_backslash() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Trailing backslash — no char to escape, but shouldn't panic
    v.validate("\\", 0)?;
    Ok(())
}

#[test]
fn validate_unmatched_parens_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Unbalanced parens: the validator doesn't check balance, just safety
    v.validate("((((", 0)?;
    v.validate("))))", 0)?;
    Ok(())
}

#[test]
fn validate_deeply_nested_safe_groups() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Deep nesting of normal groups is OK
    let open: String = "(".repeat(50);
    let close: String = ")".repeat(50);
    let pattern = format!("{open}abc{close}");
    v.validate(&pattern, 0)?;
    Ok(())
}

#[test]
fn validate_mixed_safe_constructs() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate(r"^(?:(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2}))$", 0)?;
    Ok(())
}

#[test]
fn validate_alternation_in_branch_reset() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    v.validate("(?|foo|bar|baz)", 0)?;
    Ok(())
}

#[test]
fn validate_pipe_outside_branch_reset_ok() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Many alternations in a normal group should not trigger branch limit
    let branches: String = (0..100).map(|i| format!("x{i}")).collect::<Vec<_>>().join("|");
    let pattern = format!("({branches})");
    v.validate(&pattern, 0)?;
    Ok(())
}

#[test]
fn validate_start_pos_offset_in_unicode_error() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let pattern: String = (0..51).map(|_| r"\p{L}").collect::<Vec<_>>().join("");
    let result = v.validate(&pattern, 200);
    match result {
        Err(RegexError::Syntax { offset, .. }) => {
            // Offset should be start_pos + byte index within pattern
            assert!(offset >= 200);
        }
        Ok(()) => return Err("expected error".into()),
    }
    Ok(())
}

#[test]
fn detect_nested_quantifiers_with_interleaved_literal() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Literal chars between group close and quantifier break the nesting
    assert!(!v.detect_nested_quantifiers("(a+)b+"));
    Ok(())
}

#[test]
fn code_execution_escaped_backslash_before_paren() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // \\ followed by ( — the backslash escapes the next backslash, so ( is a real group
    // "\\(?{...})" — the \\ is an escaped backslash, then (?{ is code execution
    assert!(v.detects_code_execution("\\\\(?{ code })"));
    Ok(())
}

#[test]
fn validate_complex_safe_perl_regex_without_noncapturing() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    // A realistic Perl regex with no quantifier-on-quantified-group patterns
    v.validate(r"([a-zA-Z]+)://([a-zA-Z0-9.-]+)/(\S+)", 0)?;
    Ok(())
}

#[test]
fn nested_quantifiers_lazy_modifier_still_detected() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (a+?)+ — the ? here makes the inner + lazy, but the outer + still nests
    // The ? after ) is the outer quantifier on a group with inner +
    assert!(v.detect_nested_quantifiers("(a+?)+"));
    Ok(())
}

// ── Non-capturing / lookaround groups with outer quantifier (no false positive) ──

#[test]
fn validate_lookahead_with_outer_quantifier_ok() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?=abc)+ — lookahead with outer quantifier, no inner quantifier
    v.validate("(?=abc)+", 0)?;
    Ok(())
}

#[test]
fn validate_negative_lookahead_with_outer_quantifier_ok() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    // (?!abc)+ — negative lookahead with outer quantifier, no inner quantifier
    v.validate("(?!abc)+", 0)?;
    Ok(())
}

#[test]
fn validate_lookbehind_with_outer_quantifier_ok() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?<=abc)+ — lookbehind with outer quantifier, no inner quantifier
    v.validate("(?<=abc)+", 0)?;
    Ok(())
}

#[test]
fn validate_negative_lookbehind_with_outer_quantifier_ok() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    // (?<!abc)+ — negative lookbehind with outer quantifier, no inner quantifier
    v.validate("(?<!abc)+", 0)?;
    Ok(())
}

// ── True positive: non-capturing group WITH inner quantifier still detected ──

#[test]
fn nested_quantifiers_non_capturing_with_inner_quantifier_detected()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?:a+)+ — inner quantifier a+, outer quantifier on group → nested
    assert!(v.detect_nested_quantifiers("(?:a+)+"));
    Ok(())
}

#[test]
fn nested_quantifiers_capturing_with_inner_quantifier_detected()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (a+)+ — classic nested quantifier case
    assert!(v.detect_nested_quantifiers("(a+)+"));
    Ok(())
}

#[test]
fn no_false_positive_non_capturing_without_inner_quantifier()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?:abc)+ — no inner quantifier, should NOT be flagged
    assert!(!v.detect_nested_quantifiers("(?:abc)+"));
    Ok(())
}

#[test]
fn no_false_positive_lookaround_without_inner_quantifier() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    assert!(!v.detect_nested_quantifiers("(?=abc)+"));
    assert!(!v.detect_nested_quantifiers("(?!abc)+"));
    assert!(!v.detect_nested_quantifiers("(?<=abc)+"));
    assert!(!v.detect_nested_quantifiers("(?<!abc)+"));
    Ok(())
}

// ── Bug fix: valid Perl patterns must not cause parse errors ──────────

#[test]
fn validate_accepts_non_capturing_group_with_escaped_dot() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    // (?:/\.)+ is a valid Perl regex for matching /. sequences
    v.validate(r"(?:/\.)+", 0)?;
    assert!(!v.detect_nested_quantifiers(r"(?:/\.)+"));
    Ok(())
}

#[test]
fn validate_accepts_word_class_quantifier_in_group() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (\w+)* is valid Perl (though possibly questionable practice)
    v.validate(r"(\w+)*", 0)?;
    Ok(())
}

#[test]
fn validate_accepts_non_capturing_with_quantifier() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?:pattern)+ is a perfectly normal Perl regex
    v.validate("(?:pattern)+", 0)?;
    assert!(!v.detect_nested_quantifiers("(?:pattern)+"));
    Ok(())
}

#[test]
fn validate_accepts_non_capturing_with_star() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?:pattern)* should parse cleanly
    v.validate("(?:pattern)*", 0)?;
    assert!(!v.detect_nested_quantifiers("(?:pattern)*"));
    Ok(())
}

#[test]
fn validate_accepts_non_capturing_alternation_with_quantifier()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?:foo|bar)+ is a common Perl regex idiom
    v.validate("(?:foo|bar)+", 0)?;
    assert!(!v.detect_nested_quantifiers("(?:foo|bar)+"));
    Ok(())
}

#[test]
fn validate_accepts_substitution_style_patterns() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // Pattern from: $path =~ s{(?:/\.)+}{/}g;
    v.validate(r"(?:/\.)+", 0)?;
    // Pattern with character class in non-capturing group
    v.validate(r"(?:[a-z]\d)+", 0)?;
    Ok(())
}
