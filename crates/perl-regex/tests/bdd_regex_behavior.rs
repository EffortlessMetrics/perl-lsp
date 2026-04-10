//! Behavior-driven scenarios for `perl-regex`.
//!
//! These tests focus on externally visible behavior using Given/When/Then
//! structure so feature expectations stay readable for maintainers.

use perl_regex::{RegexAnalyzer, RegexError, RegexValidator};

struct ScenarioContext {
    validator: RegexValidator,
}

impl ScenarioContext {
    fn new() -> Self {
        Self { validator: RegexValidator::new() }
    }
}

#[test]
fn scenario_safe_business_pattern_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    // Given a validator and a common structured pattern with named captures.
    let ctx = ScenarioContext::new();
    let pattern = r"^(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})$";

    // When we validate the pattern at a non-zero start offset.
    let result = ctx.validator.validate(pattern, 120);

    // Then validation succeeds.
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn scenario_nested_quantifier_is_advisory_not_fatal() -> Result<(), Box<dyn std::error::Error>> {
    // Given a pattern associated with catastrophic backtracking risk.
    let ctx = ScenarioContext::new();
    let pattern = "(a+)+";

    // When validate() and detect_nested_quantifiers() are both called.
    let validation = ctx.validator.validate(pattern, 0);
    let advisory_flag = ctx.validator.detect_nested_quantifiers(pattern);

    // Then validation still succeeds while advisory detection reports risk.
    assert!(validation.is_ok());
    assert!(advisory_flag);
    Ok(())
}

#[test]
fn scenario_unicode_property_limit_produces_offset_aware_error()
-> Result<(), Box<dyn std::error::Error>> {
    // Given a pattern with 51 Unicode property escapes (limit is 50).
    let ctx = ScenarioContext::new();
    let pattern: String = (0..51).map(|_| r"\p{L}").collect::<Vec<_>>().join("");

    // When validation starts from byte offset 37 in a larger source string.
    let result = ctx.validator.validate(&pattern, 37);

    // Then validation fails with an offset-aware syntax error.
    let err = match result {
        Ok(()) => return Err("expected unicode property overflow error".into()),
        Err(err) => err,
    };

    match err {
        RegexError::Syntax { message, offset } => {
            assert!(message.contains("Too many Unicode properties"));
            assert!(offset >= 37);
        }
    }

    Ok(())
}

#[test]
fn scenario_embedded_eval_block_is_detected_as_code_execution()
-> Result<(), Box<dyn std::error::Error>> {
    // Given patterns with and without Perl embedded eval constructs.
    let ctx = ScenarioContext::new();
    let safe = r"^\w+$";
    let dangerous = r"^(\w+)(?{ die 'boom' })$";

    // When code-execution detection runs.
    let safe_flag = ctx.validator.detects_code_execution(safe);
    let dangerous_flag = ctx.validator.detects_code_execution(dangerous);

    // Then only the dangerous pattern is flagged.
    assert!(!safe_flag);
    assert!(dangerous_flag);
    Ok(())
}

#[test]
fn scenario_named_capture_extraction_tracks_capture_indexes()
-> Result<(), Box<dyn std::error::Error>> {
    // Given mixed unnamed and named capture groups.
    let pattern = r"(prefix)(?<id>\d+)-(?<slug>[a-z]+)";

    // When named captures are extracted.
    let captures = RegexAnalyzer::extract_named_captures(pattern);

    // Then only named captures are listed with Perl-style numbering.
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0].name, "id");
    assert_eq!(captures[0].index, 2);
    assert_eq!(captures[1].name, "slug");
    assert_eq!(captures[1].index, 3);
    Ok(())
}

#[test]
fn scenario_hover_text_explains_named_captures_and_modifiers()
-> Result<(), Box<dyn std::error::Error>> {
    // Given a pattern with a named capture and multiple modifiers.
    let pattern = r"(?<ticket>[A-Z]{3}-\d+)";

    // When hover text is requested.
    let hover = RegexAnalyzer::hover_text_for_regex(pattern, "im");

    // Then the hover includes capture and modifier explanations.
    assert!(hover.contains("ticket"));
    assert!(hover.contains("case-insensitive"));
    assert!(hover.contains("multiline"));
    Ok(())
}
