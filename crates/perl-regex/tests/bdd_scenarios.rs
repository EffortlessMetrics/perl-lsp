//! BDD-style scenarios for perl-regex.
//!
//! These tests describe behavior using Given/When/Then naming so
//! safety rules are readable as user-facing acceptance criteria.

use perl_regex::RegexAnalyzer;
use perl_regex::RegexValidator;

#[test]
fn given_safe_pattern_when_validate_then_accepts_pattern() -> Result<(), Box<dyn std::error::Error>>
{
    let validator = RegexValidator::new();

    validator.validate(r"^(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})$", 0)?;

    Ok(())
}

#[test]
fn given_too_many_unicode_properties_when_validate_then_returns_error()
-> Result<(), Box<dyn std::error::Error>> {
    let validator = RegexValidator::new();
    let pattern: String = (0..51).map(|_| r"\p{L}").collect();

    let error = validator
        .validate(&pattern, 12)
        .expect_err("expected Unicode property limit validation error");

    assert!(format!("{error}").contains("Too many Unicode properties"));
    assert!(format!("{error}").contains("offset"));
    Ok(())
}

#[test]
fn given_regex_with_eval_block_when_detecting_code_execution_then_returns_true()
-> Result<(), Box<dyn std::error::Error>> {
    let validator = RegexValidator::new();

    assert!(validator.detects_code_execution(r"^(\w+)(?{ die 'boom' })$"));

    Ok(())
}

#[test]
fn given_regex_without_eval_block_when_detecting_code_execution_then_returns_false()
-> Result<(), Box<dyn std::error::Error>> {
    let validator = RegexValidator::new();

    assert!(!validator.detects_code_execution(r"^(\w+)-(\d+)$"));

    Ok(())
}

#[test]
fn given_nested_quantifier_shape_when_checking_nested_quantifiers_then_returns_true()
-> Result<(), Box<dyn std::error::Error>> {
    let validator = RegexValidator::new();

    assert!(validator.detect_nested_quantifiers(r"(?:a+)+"));

    Ok(())
}

#[test]
fn given_named_capture_pattern_when_extracting_captures_then_returns_names_and_indexes()
-> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures(r"(?<scheme>https?)://(?<host>[^/]+)");

    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "scheme");
    assert_eq!(captures[0].index, 1);
    assert_eq!(captures[1].name, "host");
    assert_eq!(captures[1].index, 2);

    Ok(())
}

#[test]
fn given_mixed_groups_when_extracting_named_captures_then_numbering_tracks_all_captures()
-> Result<(), Box<dyn std::error::Error>> {
    let captures = RegexAnalyzer::extract_named_captures(r"(\d{4})-(?<month>\d{2})-(?<day>\d{2})");

    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "month");
    assert_eq!(captures[0].index, 2);
    assert_eq!(captures[1].name, "day");
    assert_eq!(captures[1].index, 3);

    Ok(())
}

#[test]
fn given_pattern_and_modifiers_when_generating_hover_text_then_describes_captures_and_flags()
-> Result<(), Box<dyn std::error::Error>> {
    let hover = RegexAnalyzer::hover_text_for_regex(r"(?<id>\d+)", "imsxg");

    assert!(hover.contains("Named captures:"));
    assert!(hover.contains("${id} (capture 1): `\\d+`"));
    assert!(hover.contains("case-insensitive matching"));
    assert!(hover.contains("multiline mode"));
    assert!(hover.contains("single-line mode"));
    assert!(hover.contains("extended mode"));
    assert!(hover.contains("global: match all occurrences"));

    Ok(())
}
