//! Property-based tests for `perl-regex` validation functions.
//!
//! Verifies that all public APIs are panic-free on arbitrary input and that
//! key invariants hold: determinism, capture-index monotonicity, and
//! conservative code-execution detection.

use perl_regex::{RegexAnalyzer, RegexValidator};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A generator for printable ASCII strings (no NUL bytes), keeping lengths
/// short so shrinking stays fast.
fn ascii_pattern() -> impl Strategy<Value = String> {
    prop::collection::vec(0x20u8..0x7fu8, 0..128)
        .prop_map(|bytes| String::from_utf8(bytes).unwrap())
}

/// A generator for typical regex modifier strings.
fn modifiers() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(vec!['i', 'm', 's', 'x', 'g']), 0..4)
        .prop_map(|chars| chars.into_iter().collect::<String>())
}

// ---------------------------------------------------------------------------
// Panic-freedom
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        ..ProptestConfig::default()
    })]

    /// `validate()` must never panic on arbitrary printable ASCII.
    #[test]
    fn validate_never_panics(s in ascii_pattern(), offset in 0usize..1024) {
        let _ = RegexValidator::new().validate(&s, offset);
    }

    /// `detects_code_execution()` must never panic.
    #[test]
    fn code_execution_never_panics(s in ascii_pattern()) {
        let _ = RegexValidator::new().detects_code_execution(&s);
    }

    /// `detect_nested_quantifiers()` must never panic.
    #[test]
    fn nested_quantifiers_never_panics(s in ascii_pattern()) {
        let _ = RegexValidator::new().detect_nested_quantifiers(&s);
    }

    /// `extract_named_captures()` must never panic.
    #[test]
    fn extract_captures_never_panics(s in ascii_pattern()) {
        let _ = RegexAnalyzer::extract_named_captures(&s);
    }

    /// `hover_text_for_regex()` must never panic on arbitrary pattern + modifiers.
    #[test]
    fn hover_text_never_panics(s in ascii_pattern(), m in modifiers()) {
        let _ = RegexAnalyzer::hover_text_for_regex(&s, &m);
    }
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Calling `validate()` twice on the same input returns the same result.
    #[test]
    fn validate_is_deterministic(s in ascii_pattern(), offset in 0usize..1024) {
        let v = RegexValidator::new();
        let r1 = v.validate(&s, offset);
        let r2 = v.validate(&s, offset);
        prop_assert_eq!(r1, r2);
    }

    /// `detects_code_execution()` is deterministic.
    #[test]
    fn code_execution_is_deterministic(s in ascii_pattern()) {
        let v = RegexValidator::new();
        prop_assert_eq!(v.detects_code_execution(&s), v.detects_code_execution(&s));
    }

    /// `detect_nested_quantifiers()` is deterministic.
    #[test]
    fn nested_quantifiers_is_deterministic(s in ascii_pattern()) {
        let v = RegexValidator::new();
        prop_assert_eq!(v.detect_nested_quantifiers(&s), v.detect_nested_quantifiers(&s));
    }

    /// Named capture indices are strictly monotonically increasing (1-based).
    #[test]
    fn capture_indices_are_monotonic(s in ascii_pattern()) {
        let caps = RegexAnalyzer::extract_named_captures(&s);
        for window in caps.windows(2) {
            prop_assert!(
                window[1].index > window[0].index,
                "indices not strictly increasing: {} then {} in {:?}",
                window[0].index,
                window[1].index,
                s,
            );
        }
    }

    /// All named captures have non-empty names.
    #[test]
    fn capture_names_are_non_empty(s in ascii_pattern()) {
        for cap in RegexAnalyzer::extract_named_captures(&s) {
            prop_assert!(!cap.name.is_empty(), "empty capture name in {:?}", s);
        }
    }

    /// Capture indices start at 1 or higher.
    #[test]
    fn capture_indices_start_at_one(s in ascii_pattern()) {
        let caps = RegexAnalyzer::extract_named_captures(&s);
        if let Some(first) = caps.first() {
            prop_assert!(first.index >= 1, "capture index < 1 in {:?}", s);
        }
    }
}

// ---------------------------------------------------------------------------
// Code-execution detection: conservative invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Patterns that contain neither `(?{` nor `(??{` are never flagged.
    /// This guards against false positives on arbitrary ASCII.
    #[test]
    fn no_code_constructs_means_no_detection(
        // Allow only ASCII printable except the sequence-forming characters
        // by generating from a character set that cannot form (?{ or (??{.
        s in "[A-Za-z0-9 _.,;:'\"!@#%&*\\[\\]<>/|^~`=+-]{0,80}"
    ) {
        // The strategy excludes '(' and '{' so (?{ / (??{ can't appear.
        prop_assert!(!RegexValidator::new().detects_code_execution(&s));
    }

    /// A pattern with `(?{` embedded always triggers detection.
    #[test]
    fn explicit_code_block_always_detected(
        prefix in "[A-Za-z0-9]{0,10}",
        suffix in "[A-Za-z0-9]{0,10}",
        inner in "[A-Za-z0-9 ]{0,20}",
    ) {
        let s = format!("{prefix}(?{{{inner}}}{suffix}");
        prop_assert!(
            RegexValidator::new().detects_code_execution(&s),
            "should detect (?{{...}} in {:?}",
            s
        );
    }

    /// A pattern with `(??{` always triggers detection.
    #[test]
    fn deferred_code_block_always_detected(
        prefix in "[A-Za-z0-9]{0,10}",
        suffix in "[A-Za-z0-9]{0,10}",
        inner in "[A-Za-z0-9 ]{0,20}",
    ) {
        let s = format!("{prefix}(??{{{inner}}}{suffix}");
        prop_assert!(
            RegexValidator::new().detects_code_execution(&s),
            "should detect (??{{...}} in {:?}",
            s
        );
    }
}
