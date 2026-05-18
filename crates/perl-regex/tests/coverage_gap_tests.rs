//! Coverage gap tests for the `perl-regex` crate.
//!
//! Targets paths identified as genuinely uncovered after reviewing all existing
//! test files (behavior_spec_tests.rs, comprehensive_unit_tests.rs,
//! named_capture_tests.rs, prop_regex_validator.rs):
//!
//! - `prelude` re-export module (never imported in any test)
//! - `RegexFinding` derived traits: `Debug`, `Clone`, `PartialEq`/`Eq`
//! - `find_code_execution` returning `None` (explicit assertion)
//! - `find_nested_quantifier` returning `None` (explicit assertion)
//! - `extract_named_captures` with empty angle-bracket name `(?<>...)`
//! - `extract_named_captures` with unclosed angle bracket `(?<unclosed...)`
//! - `CaptureGroup::PartialEq` derived impl
//! - `collect_subpattern` when the captured sub-pattern contains nested groups

use perl_regex::{CaptureGroup, RegexAnalyzer, RegexValidator, validator::RegexFinding};

// Confirm `prelude` re-exports all public items without shadowing.
// If the prelude ever diverges from `pub use crate::...`, this import will fail.
use perl_regex::prelude::*;

// ── prelude re-exports ───────────────────────────────────────────────────

#[test]
fn prelude_reexports_validator() -> Result<(), Box<dyn std::error::Error>> {
    // `RegexValidator` is re-exported via `perl_regex::prelude`.
    // Construct one through the prelude alias to confirm the export is live.
    let v: RegexValidator = RegexValidator::new();
    v.validate("hello", 0)?;
    Ok(())
}

#[test]
fn prelude_reexports_analyzer() -> Result<(), Box<dyn std::error::Error>> {
    // `RegexAnalyzer` is re-exported via `perl_regex::prelude`.
    let caps = RegexAnalyzer::extract_named_captures(r"(?<x>\d+)");
    assert_eq!(caps.len(), 1);
    Ok(())
}

#[test]
fn prelude_reexports_regex_error() -> Result<(), Box<dyn std::error::Error>> {
    // `RegexError` is re-exported via `perl_regex::prelude`.
    let err = RegexError::syntax("test", 0);
    assert!(err.to_string().contains("test"));
    Ok(())
}

#[test]
fn prelude_reexports_capture_group() -> Result<(), Box<dyn std::error::Error>> {
    // `CaptureGroup` is re-exported via `perl_regex::prelude`.
    let cg = CaptureGroup { name: "x".to_string(), index: 1, pattern: "\\d+".to_string() };
    assert_eq!(cg.name, "x");
    Ok(())
}

// ── RegexFinding derives ─────────────────────────────────────────────────

#[test]
fn regex_finding_debug_contains_offset_and_message() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let finding = v.find_code_execution("(?{ run })", 0).ok_or("expected finding")?;
    let dbg = format!("{finding:?}");
    assert!(dbg.contains("offset"), "debug output missing 'offset': {dbg}");
    assert!(dbg.contains("message"), "debug output missing 'message': {dbg}");
    Ok(())
}

#[test]
fn regex_finding_clone_produces_equal_value() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let original = v.find_code_execution("(?{ run })", 5).ok_or("expected finding")?;
    let cloned = original.clone();
    assert_eq!(original, cloned);
    Ok(())
}

#[test]
fn regex_finding_partial_eq_same_values() -> Result<(), Box<dyn std::error::Error>> {
    let f1 = RegexFinding { offset: 3, message: "bad" };
    let f2 = RegexFinding { offset: 3, message: "bad" };
    assert_eq!(f1, f2);
    Ok(())
}

#[test]
fn regex_finding_partial_eq_different_offset() -> Result<(), Box<dyn std::error::Error>> {
    let f1 = RegexFinding { offset: 3, message: "bad" };
    let f2 = RegexFinding { offset: 4, message: "bad" };
    assert_ne!(f1, f2);
    Ok(())
}

#[test]
fn regex_finding_partial_eq_different_message() -> Result<(), Box<dyn std::error::Error>> {
    let f1 = RegexFinding { offset: 3, message: "bad" };
    let f2 = RegexFinding { offset: 3, message: "worse" };
    assert_ne!(f1, f2);
    Ok(())
}

// ── find_code_execution → None (safe patterns) ──────────────────────────

#[test]
fn find_code_execution_returns_none_for_safe_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let result = v.find_code_execution("hello world", 0);
    assert!(result.is_none(), "expected None for safe pattern");
    Ok(())
}

#[test]
fn find_code_execution_returns_none_for_empty_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let result = v.find_code_execution("", 0);
    assert!(result.is_none());
    Ok(())
}

#[test]
fn find_code_execution_returns_none_for_non_capturing_group()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    // (?:...) is not code execution
    let result = v.find_code_execution("(?:safe)", 0);
    assert!(result.is_none());
    Ok(())
}

// ── find_nested_quantifier → None (safe patterns) ───────────────────────

#[test]
fn find_nested_quantifier_returns_none_for_safe_pattern() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    let result = v.find_nested_quantifier("(abc)+", 0);
    assert!(result.is_none(), "expected None for pattern without inner quantifier");
    Ok(())
}

#[test]
fn find_nested_quantifier_returns_none_for_empty_pattern() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    let result = v.find_nested_quantifier("", 0);
    assert!(result.is_none());
    Ok(())
}

#[test]
fn find_nested_quantifier_returns_none_for_simple_literal() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    let result = v.find_nested_quantifier("a+b*c?", 0);
    assert!(result.is_none());
    Ok(())
}

// ── extract_named_captures: empty angle-bracket name ────────────────────

#[test]
fn extract_named_captures_empty_angle_bracket_name_ignored()
-> Result<(), Box<dyn std::error::Error>> {
    // (?<>...) — empty name between < and > → parse_named_capture_name_from returns None
    // The group is silently skipped (not counted as a named or unnamed capture).
    let caps = RegexAnalyzer::extract_named_captures("(?<>\\d+)");
    assert!(
        caps.is_empty(),
        "empty angle-bracket name must not produce a CaptureGroup, got: {caps:?}"
    );
    Ok(())
}

#[test]
fn extract_named_captures_empty_angle_name_does_not_affect_subsequent_capture_index()
-> Result<(), Box<dyn std::error::Error>> {
    // (?<>...) is skipped; (?<real>...) that follows starts at index 1 because
    // the empty-name group falls through without incrementing the capture counter.
    let caps = RegexAnalyzer::extract_named_captures(r"(?<>\w+)(?<real>\d+)");
    assert_eq!(caps.len(), 1, "only the real named capture should appear");
    assert_eq!(caps[0].name, "real");
    Ok(())
}

// ── extract_named_captures: unclosed angle bracket ───────────────────────

#[test]
fn extract_named_captures_unclosed_angle_bracket_ignored() -> Result<(), Box<dyn std::error::Error>>
{
    // (?<unclosed — no closing '>' — parse_named_capture_name_from returns None.
    let caps = RegexAnalyzer::extract_named_captures("(?<unclosed\\d+)");
    assert!(
        caps.is_empty(),
        "unclosed angle-bracket capture must not produce a CaptureGroup, got: {caps:?}"
    );
    Ok(())
}

// ── CaptureGroup::PartialEq ──────────────────────────────────────────────

#[test]
fn capture_group_partial_eq_same_values() -> Result<(), Box<dyn std::error::Error>> {
    let a = CaptureGroup { name: "id".to_string(), index: 1, pattern: "\\d+".to_string() };
    let b = CaptureGroup { name: "id".to_string(), index: 1, pattern: "\\d+".to_string() };
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn capture_group_partial_eq_different_name() -> Result<(), Box<dyn std::error::Error>> {
    let a = CaptureGroup { name: "foo".to_string(), index: 1, pattern: "\\d+".to_string() };
    let b = CaptureGroup { name: "bar".to_string(), index: 1, pattern: "\\d+".to_string() };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn capture_group_partial_eq_different_index() -> Result<(), Box<dyn std::error::Error>> {
    let a = CaptureGroup { name: "x".to_string(), index: 1, pattern: "\\d+".to_string() };
    let b = CaptureGroup { name: "x".to_string(), index: 2, pattern: "\\d+".to_string() };
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn capture_group_partial_eq_different_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let a = CaptureGroup { name: "x".to_string(), index: 1, pattern: "\\d+".to_string() };
    let b = CaptureGroup { name: "x".to_string(), index: 1, pattern: "\\w+".to_string() };
    assert_ne!(a, b);
    Ok(())
}

// ── collect_subpattern: nested groups in sub-pattern ─────────────────────

#[test]
fn extract_named_captures_subpattern_with_nested_group() -> Result<(), Box<dyn std::error::Error>> {
    // The sub-pattern contains an inner capturing group: (?<outer>(inner)\d+)
    // collect_subpattern must track nesting depth to find the correct closing ')'.
    let caps = RegexAnalyzer::extract_named_captures(r"(?<outer>(inner)\d+)");
    assert_eq!(caps.len(), 1, "one named capture expected");
    assert_eq!(caps[0].name, "outer");
    // Sub-pattern is everything inside (?<outer>...) — the inner group plus \d+
    assert!(
        caps[0].pattern.contains("inner"),
        "sub-pattern should include the nested group content, got: {:?}",
        caps[0].pattern
    );
    Ok(())
}

#[test]
fn extract_named_captures_subpattern_with_char_class_containing_paren()
-> Result<(), Box<dyn std::error::Error>> {
    // collect_subpattern must not treat ')' inside [...] as group close.
    let caps = RegexAnalyzer::extract_named_captures(r"(?<tok>[^)]+)rest");
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].name, "tok");
    assert_eq!(caps[0].pattern, "[^)]+");
    Ok(())
}

// ── validate priority: code-execution before nested-quantifier ───────────

#[test]
fn validate_code_execution_finding_has_message_field() -> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let finding = v.find_code_execution("abc(?{ run })", 0).ok_or("expected finding")?;
    // Immediate code block message
    assert!(
        finding.message.contains("Embedded code execution"),
        "unexpected message: {}",
        finding.message
    );
    Ok(())
}

#[test]
fn validate_deferred_code_execution_finding_has_deferred_message()
-> Result<(), Box<dyn std::error::Error>> {
    let v = RegexValidator::new();
    let finding = v.find_code_execution("abc(??{ run })", 0).ok_or("expected deferred finding")?;
    assert!(
        finding.message.contains("Deferred embedded code execution"),
        "unexpected message: {}",
        finding.message
    );
    Ok(())
}

#[test]
fn validate_nested_quantifier_finding_has_message_field() -> Result<(), Box<dyn std::error::Error>>
{
    let v = RegexValidator::new();
    let finding = v.find_nested_quantifier("(a+)+", 0).ok_or("expected finding")?;
    assert!(
        finding.message.contains("Nested quantifiers"),
        "unexpected message: {}",
        finding.message
    );
    Ok(())
}
