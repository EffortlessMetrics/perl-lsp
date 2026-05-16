//! Targeted branch-coverage tests for perl-regex.
//!
//! These tests drive branch arms that are structurally reachable but not
//! exercised by any other test suite.  Each test includes a comment
//! identifying the exact source site it covers.

use perl_regex::{RegexAnalyzer, RegexValidator};

// ── analyzer/capture.rs — negative lookbehind in extract_named_captures ──
//
// capture.rs line 43: `bytes[i] == b'='  || bytes[i] == b'!'`
// The `b'='` arm (positive lookbehind `(?<=...)`) is exercised by existing
// tests.  The `b'!'` arm (negative lookbehind `(?<!...)`) is only reached
// when the character after `(?<` is `!`.  Add a test that puts a negative
// lookbehind before a named capture so `extract_named_captures` parses both
// the lookbehind (covering the `b'!'` arm) and the subsequent named group.

#[test]
fn extract_named_captures_negative_lookbehind_not_counted_as_capture()
-> Result<(), Box<dyn std::error::Error>> {
    // (?<!foo) is a negative lookbehind: NOT a named capture.
    // (?<word>\w+) is the actual named capture.
    // covers: capture.rs line 43 col 64 — bytes[i] == b'!' branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures(r"(?<!foo)(?<word>\w+)");
    assert_eq!(caps.len(), 1, "negative lookbehind must not be counted as a capture");
    assert_eq!(caps[0].name, "word");
    assert_eq!(caps[0].index, 1);
    Ok(())
}

#[test]
fn extract_named_captures_negative_lookbehind_only_pattern()
-> Result<(), Box<dyn std::error::Error>> {
    // A pattern with only a negative lookbehind produces no captures.
    // covers: capture.rs line 43 col 64 — bytes[i] == b'!' branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures(r"(?<!prefix)\w+");
    assert!(caps.is_empty(), "negative lookbehind alone must produce no capture");
    Ok(())
}

// ── analyzer/capture.rs — escape inside char class in collect_subpattern ─
//
// capture.rs line 99: `bytes[i] == b'\\'` inside the char-class inner loop
// of `collect_subpattern`.  The existing test only uses `[^)]+` which has
// no backslash inside the class.  Using `[\+]` puts a real escape inside
// the class within the named capture's subpattern.

#[test]
fn extract_named_captures_subpattern_with_escape_in_char_class()
-> Result<(), Box<dyn std::error::Error>> {
    // Named capture whose subpattern contains [\\+] — escape inside char class.
    // covers: capture.rs line 99 col 20 — bytes[i] == b'\\' branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures(r"(?<tok>[\+\-]+)");
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].name, "tok");
    assert!(
        caps[0].pattern.contains(r"\+"),
        "subpattern should preserve escape sequences: got {:?}",
        caps[0].pattern,
    );
    Ok(())
}

// ── analyzer/capture.rs — nested group inside collect_subpattern ──────────
//
// capture.rs line 111: `bytes[i] == b'('` increments the subpattern depth
// counter.  No existing test has a nested group inside a named capture's
// subpattern, so this branch is never True.  Using `(?<outer>a(?:inner)b)`
// puts a literal `(` inside the subpattern.

#[test]
fn extract_named_captures_nested_group_in_subpattern() -> Result<(), Box<dyn std::error::Error>> {
    // Named capture whose subpattern contains a nested non-capturing group.
    // covers: capture.rs line 111 col 12 — bytes[i] == b'(' branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures(r"(?<outer>a(?:inner)b)");
    assert_eq!(caps.len(), 1, "only the outer named capture should be extracted");
    assert_eq!(caps[0].name, "outer");
    assert_eq!(caps[0].index, 1);
    // The subpattern should include the nested group content.
    assert!(
        caps[0].pattern.contains("inner"),
        "subpattern should capture nested group content: got {:?}",
        caps[0].pattern,
    );
    Ok(())
}

#[test]
fn extract_named_captures_deeply_nested_subpattern() -> Result<(), Box<dyn std::error::Error>> {
    // Named capture with multiple levels of nesting in the subpattern.
    // covers: capture.rs line 111 col 12 — bytes[i] == b'(' branch (multiple hits)
    let caps = RegexAnalyzer::extract_named_captures(r"(?<url>https?://([\w.]+)/(\S+))");
    assert_eq!(caps.len(), 1, "only the outer named capture should be returned");
    assert_eq!(caps[0].name, "url");
    Ok(())
}

// ── analyzer/parser.rs — truncated patterns returning None ───────────────
//
// parser.rs `parse_named_capture_name_from` line 26: `start >= bytes.len()`
// returns None when the pattern ends immediately after `(?<`.
//
// parser.rs line 33: `i == start` returns None for an empty angle-bracket
// name `(?<>...)`.
//
// parser.rs line 33: `i >= bytes.len()` returns None when the closing `>`
// is never found (pattern truncated inside the name).

#[test]
fn extract_named_captures_truncated_after_angle_open() -> Result<(), Box<dyn std::error::Error>> {
    // Pattern ends immediately after `(?<` — the name parser returns None.
    // covers: parser.rs line 26 col 8 — start >= bytes.len() branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures("(?<");
    assert!(caps.is_empty(), "truncated (?< must not produce a capture");
    Ok(())
}

#[test]
fn extract_named_captures_empty_angle_bracket_name() -> Result<(), Box<dyn std::error::Error>> {
    // `(?<>...)` has an empty name — `parse_named_capture_name_from` sees
    // i == start at the `>` and returns None.
    // covers: parser.rs line 33 col 8 — i == start branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures("(?<>\\d+)");
    assert!(caps.is_empty(), "empty angle-bracket name must not produce a capture");
    Ok(())
}

#[test]
fn extract_named_captures_unclosed_angle_bracket_name() -> Result<(), Box<dyn std::error::Error>> {
    // `(?<foo` — closing `>` is never found, parser exhausts bytes.
    // covers: parser.rs line 33 col 22 — i >= bytes.len() branch (True arm)
    let caps = RegexAnalyzer::extract_named_captures("(?<unclosed_name");
    assert!(caps.is_empty(), "unclosed angle-bracket name must not produce a capture");
    Ok(())
}

// ── Validate integration: code-execution and nested-quantifier paths ──────
//
// validator/mod.rs lines 41 and 44: `validate()` has two `if let Some(...)` arms
// that return early when code execution or nested quantifiers are found.
// The proptest binary calls `validate()` on random ASCII which statistically
// rarely triggers these paths.  Add targeted `validate()` tests here so
// this binary also covers those early-return arms.

#[test]
fn validate_returns_error_for_code_execution_pattern() -> Result<(), Box<dyn std::error::Error>> {
    // covers: validator/mod.rs line 41 — find_code_execution returns Some
    let v = RegexValidator::new();
    let Err(err) = v.validate("(?{ system('ls') })", 0) else {
        return Err("code-execution pattern must fail validation".into());
    };
    let msg = err.to_string();
    assert!(
        msg.contains("code execution") || msg.contains("Embedded code"),
        "error should mention code execution: {msg}"
    );
    Ok(())
}

#[test]
fn validate_returns_error_for_deferred_code_execution_pattern()
-> Result<(), Box<dyn std::error::Error>> {
    // covers: validator/mod.rs line 41 — find_code_execution returns Some (deferred kind)
    let v = RegexValidator::new();
    let Err(err) = v.validate("(??{ $code })", 0) else {
        return Err("deferred code-execution pattern must fail validation".into());
    };
    let msg = err.to_string();
    assert!(
        msg.contains("code execution") || msg.contains("Deferred"),
        "error should mention deferred code execution: {msg}"
    );
    Ok(())
}

#[test]
fn validate_returns_error_for_nested_quantifier_pattern() -> Result<(), Box<dyn std::error::Error>>
{
    // covers: validator/mod.rs line 44 — find_nested_quantifier returns Some
    let v = RegexValidator::new();
    let Err(err) = v.validate("(a+)+", 0) else {
        return Err("nested-quantifier pattern must fail validation".into());
    };
    let msg = err.to_string();
    assert!(
        msg.contains("quantifier") || msg.contains("backtracking"),
        "error should mention quantifiers: {msg}"
    );
    Ok(())
}

// ── complexity.rs — escape inside char class ──────────────────────────────
//
// complexity.rs line 49: `bytes[i] == b'\\'` inside the char-class skip
// loop.  This is covered by the unit-test binary but missed by the proptest
// binary.  Adding a targeted test here ensures this binary (branch_coverage_tests)
// also hits it.

#[test]
fn validate_pattern_with_escape_inside_char_class() -> Result<(), Box<dyn std::error::Error>> {
    // Pattern `[\+\-]+` has backslashes inside the character class.
    // covers: complexity.rs line 49 — bytes[i] == b'\\' (True arm inside [])
    let v = RegexValidator::new();
    v.validate(r"[\+\-]+", 0)?;
    Ok(())
}

#[test]
fn validate_pattern_with_escaped_bracket_in_char_class() -> Result<(), Box<dyn std::error::Error>> {
    // `[\]]` has `\]` inside a char class — triggers the escape skip.
    // covers: complexity.rs line 49 — bytes[i] == b'\\' (True arm inside [])
    let v = RegexValidator::new();
    v.validate(r"[\]]", 0)?;
    Ok(())
}

// ── complexity.rs — negative lookbehind (b'!') ────────────────────────────
//
// complexity.rs line 64: `bytes[i] == b'='  || bytes[i] == b'!'`
// Positive lookbehind `(?<=...)` hits `b'='`.  Negative lookbehind
// `(?<!...)` hits `b'!'` via the short-circuit second arm.

#[test]
fn validate_negative_lookbehind_covers_bang_branch() -> Result<(), Box<dyn std::error::Error>> {
    // covers: complexity.rs line 64 col 68 — bytes[i] == b'!' (True arm)
    let v = RegexValidator::new();
    v.validate(r"(?<!foo)bar", 0)?;
    Ok(())
}

#[test]
fn validate_multiple_negative_lookbehinds() -> Result<(), Box<dyn std::error::Error>> {
    // Multiple negative lookbehinds to exercise the branch repeatedly.
    // covers: complexity.rs line 64 col 68 — bytes[i] == b'!' (True arm, multiple)
    let v = RegexValidator::new();
    v.validate(r"(?<!a)(?<!b)(?<!c)\w+", 0)?;
    Ok(())
}

// ── nested_quantifier.rs — escape inside char class ───────────────────────
//
// nested_quantifier.rs line 20: `bytes[i] == b'\\'` inside the char-class
// skip loop.  Covered by unit-test binary but not by the proptest binary.

#[test]
fn detect_nested_quantifiers_escape_inside_char_class_not_flagged()
-> Result<(), Box<dyn std::error::Error>> {
    // `([\+]+)+` has `\+` inside a char class inside a quantified group.
    // covers: nested_quantifier.rs line 20 — bytes[i] == b'\\' (True arm inside [])
    let v = RegexValidator::new();
    // [\+] is a char class with a literal +, so no nested quantifier
    assert!(!v.detect_nested_quantifiers(r"([\+])+"));
    Ok(())
}

#[test]
fn detect_nested_quantifiers_escape_backslash_in_char_class()
-> Result<(), Box<dyn std::error::Error>> {
    // `([\\]+)+` has `\\` inside a char class — double backslash.
    // covers: nested_quantifier.rs line 20 — bytes[i] == b'\\' (True arm inside [])
    let v = RegexValidator::new();
    // [\\] matches a literal backslash; no nested quantifier inside the class
    assert!(!v.detect_nested_quantifiers(r"([\\])+"));
    Ok(())
}
