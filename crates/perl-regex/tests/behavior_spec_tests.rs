//! Behavior-driven tests for `perl-regex`.
//!
//! These scenarios capture externally observable behavior with
//! Given/When/Then structure so consumers can reason about validator
//! and analyzer contracts at a higher level than unit-level edge cases.

use perl_regex::{RegexAnalyzer, RegexError, RegexValidator};

#[test]
fn scenario_safe_pattern_passes_validation() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a validator and a common safe Perl regex pattern.
    let validator = RegexValidator::new();
    let pattern = r"^(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})$";

    // When: the pattern is validated.
    let result = validator.validate(pattern, 0);

    // Then: validation succeeds.
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn scenario_excessive_unicode_properties_returns_offset_error()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a validator and a pattern that exceeds unicode-property safety limits.
    let validator = RegexValidator::new();
    let pattern: String = (0..51).map(|_| r"\p{L}").collect::<Vec<_>>().join("");

    // When: the pattern is validated with a non-zero starting source position.
    let result = validator.validate(&pattern, 120);

    // Then: validation fails with an offset-aware error message.
    match result {
        Err(RegexError::Syntax { message, offset }) => {
            assert!(message.contains("Too many Unicode properties"));
            assert!(offset >= 120);
        }
        Ok(()) => return Err("expected Unicode property limit error".into()),
    }

    Ok(())
}

#[test]
fn scenario_nested_quantifier_is_advisory_not_fatal() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a nested-quantifier pattern that may cause catastrophic backtracking.
    let validator = RegexValidator::new();
    let pattern = "(a+)+";

    // When: validating and separately asking for advisory nested-quantifier detection.
    let validation_result = validator.validate(pattern, 0);
    let advisory_detected = validator.detect_nested_quantifiers(pattern);

    // Then: validation remains non-fatal, while advisory detection flags the risk.
    assert!(validation_result.is_ok());
    assert!(advisory_detected);
    Ok(())
}

#[test]
fn scenario_embedded_code_is_detected() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a regex containing Perl embedded code execution syntax.
    let validator = RegexValidator::new();
    let pattern = r"^(\w+)(?{ die 'unsafe' })$";

    // When: scanning for code execution constructs.
    let executes_code = validator.detects_code_execution(pattern);

    // Then: the scanner reports code execution presence.
    assert!(executes_code);
    Ok(())
}

#[test]
fn scenario_named_capture_indexes_follow_capture_order() -> Result<(), Box<dyn std::error::Error>> {
    // Given: mixed unnamed and named captures.
    let pattern = r"(prefix)(?<id>\d+)(?:-)(?<suffix>\w+)";

    // When: extracting named captures.
    let captures = RegexAnalyzer::extract_named_captures(pattern);

    // Then: named captures preserve left-to-right capture numbering.
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 2);
    assert_eq!(captures[1].name, "suffix");
    assert_eq!(captures[1].index, 3);
    Ok(())
}

#[test]
fn scenario_hover_text_summarizes_captures_and_modifiers() -> Result<(), Box<dyn std::error::Error>>
{
    // Given: a regex with a named capture and two modifiers.
    let pattern = r"(?<word>\w+)";
    let modifiers = "gi";

    // When: generating hover text for IDE presentation.
    let hover = RegexAnalyzer::hover_text_for_regex(pattern, modifiers);

    // Then: output includes capture details and modifier semantics.
    assert!(hover.contains("Named captures"));
    assert!(hover.contains("word"));
    assert!(hover.contains("case-insensitive"));
    assert!(hover.contains("global"));
    Ok(())
}

#[test]
fn scenario_hover_text_lists_extended_perl_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    // Given: modifiers that are valid in Perl but were historically omitted from notes.
    let hover = RegexAnalyzer::hover_text_for_regex(r"(?<word>\w+)", "anu");

    // Then: hover text explains the Perl-specific semantics.
    assert!(hover.contains("ASCII-safe character classes"));
    assert!(hover.contains("non-capturing by default"));
    assert!(hover.contains("Unicode character semantics"));
    Ok(())
}

#[test]
fn scenario_hover_text_deduplicates_and_reports_unknown_modifiers()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: repeated modifiers and unknown flags.
    let hover = RegexAnalyzer::hover_text_for_regex(r"\w+", "iizzz");

    // Then: duplicate notes are removed and unknown flags are reported once.
    assert_eq!(hover.matches("case-insensitive matching").count(), 1);
    assert!(hover.contains("Unknown modifiers: `z`"));
    Ok(())
}

#[test]
fn scenario_hover_text_covers_substitution_specific_modifiers()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: modifiers that are only meaningful on s///.
    let hover = RegexAnalyzer::hover_text_for_regex(r"foo", "re");

    // Then: hover explains non-destructive result (r) and eval replacement (e).
    assert!(hover.contains("non-destructive substitution result"), "expected /r description");
    assert!(hover.contains("evaluate replacement as code"), "expected /e description");
    Ok(())
}

#[test]
fn scenario_hover_text_deduplicates_repeated_known_modifier()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: the same known modifier repeated twice (e.g. //xx for Perl 5.26 extended).
    let hover = RegexAnalyzer::hover_text_for_regex(r"\s+", "xx");

    // Then: the modifier note appears exactly once, not duplicated.
    assert_eq!(
        hover.matches("extended mode: whitespace and comments allowed").count(),
        1,
        "duplicate /x should produce exactly one note"
    );
    // And there must be no Unknown modifiers section (x is known).
    assert!(
        !hover.contains("Unknown modifiers"),
        "repeated known modifier must not appear in unknown section"
    );
    Ok(())
}
