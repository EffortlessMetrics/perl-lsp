//! Perl regex validation and analysis.

pub mod analyzer;
pub mod error;
pub mod prelude;
pub mod validator;

mod syntax;

pub use analyzer::{CaptureGroup, RegexAnalyzer};
pub use error::RegexError;
pub use validator::RegexValidator;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::RegexValidationConfig;

    // --- RegexError ---

    #[test]
    fn regex_error_syntax_stores_message_and_offset() {
        let err = RegexError::syntax("unexpected char", 7);
        match &err {
            RegexError::Syntax { message, offset } => {
                assert_eq!(message, "unexpected char");
                assert_eq!(*offset, 7);
            }
        }
        assert!(err.to_string().contains("7"));
        assert!(err.to_string().contains("unexpected char"));
    }

    #[test]
    fn regex_error_implements_clone_and_partialeq() {
        let e1 = RegexError::syntax("msg", 3);
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    // --- RegexValidator::validate (valid patterns) ---

    #[test]
    fn validate_simple_pattern_ok() {
        let v = RegexValidator::new();
        assert!(v.validate("hello", 0).is_ok());
        assert!(v.validate("", 0).is_ok());
        assert!(v.validate("(a|b)+", 0).is_ok());
    }

    #[test]
    fn validate_unicode_property_within_limit_ok() {
        let v = RegexValidator::new();
        // 50 unicode properties is the limit
        let pattern = r"\p{L}".repeat(50);
        assert!(v.validate(&pattern, 0).is_ok());
    }

    #[test]
    fn validate_too_many_unicode_properties_errors() {
        let v = RegexValidator::new();
        let pattern = r"\p{L}".repeat(51);
        let err = v.validate(&pattern, 0).unwrap_err();
        assert!(err.to_string().contains("Unicode"));
    }

    #[test]
    fn validate_unicode_property_error_reports_configured_limit() {
        let config = RegexValidationConfig {
            max_nesting: 10,
            max_unicode_properties: 1,
            max_branch_reset_branches: 50,
        };
        let v = RegexValidator::with_config(config);
        let result = v.validate(r"\p{L}\p{N}", 0);
        let message = result.err().map(|err| err.to_string()).unwrap_or_default();
        assert!(message.contains("max 1"));
    }

    #[test]
    fn validate_unicode_property_offset_propagated() {
        let v = RegexValidator::new();
        let prefix = "x";
        let pattern = format!("{}{}", prefix, r"\p{L}".repeat(51));
        let err = v.validate(&pattern, 10).unwrap_err();
        // The reported offset should be >= 10 (start_pos)
        match err {
            RegexError::Syntax { offset, .. } => assert!(offset >= 10),
        }
    }

    #[test]
    fn validate_lookbehind_within_limit_ok() {
        let v = RegexValidator::new();
        // 10 is the limit; 9 nested lookbehinds should be fine
        let mut pattern = String::from("foo");
        for _ in 0..9 {
            pattern = format!("(?<={})", pattern);
        }
        assert!(v.validate(&pattern, 0).is_ok());
    }

    #[test]
    fn validate_lookbehind_nesting_too_deep_errors() {
        let v = RegexValidator::new();
        // Build 11 nested lookbehinds to exceed the depth limit of 10
        let mut pattern = String::from("a");
        for _ in 0..11 {
            pattern = format!("(?<={})", pattern);
        }
        let err = v.validate(&pattern, 0).unwrap_err();
        assert!(err.to_string().contains("lookbehind") || err.to_string().contains("nesting"));
    }

    #[test]
    fn validate_branch_reset_nesting_too_deep_errors() {
        let v = RegexValidator::new();
        let mut pattern = String::from("a");
        for _ in 0..11 {
            pattern = format!("(?|{})", pattern);
        }
        let err = v.validate(&pattern, 0).unwrap_err();
        assert!(err.to_string().contains("branch reset") || err.to_string().contains("nesting"));
    }

    #[test]
    fn validate_too_many_branches_in_reset_group_errors() {
        let v = RegexValidator::new();
        // 51 alternatives in one (?| ... ) group exceeds max 50 branches
        let alts = (0u32..51).map(|i| format!("a{i}")).collect::<Vec<_>>().join("|");
        let pattern = format!("(?|{alts})");
        let err = v.validate(&pattern, 0).unwrap_err();
        assert!(err.to_string().contains("branch") || err.to_string().contains("50"));
    }

    #[test]
    fn validate_branch_reset_error_reports_configured_limit() {
        let config = RegexValidationConfig {
            max_nesting: 10,
            max_unicode_properties: 50,
            max_branch_reset_branches: 2,
        };
        let v = RegexValidator::with_config(config);
        let result = v.validate("(?|a|b|c)", 0);
        let message = result.err().map(|err| err.to_string()).unwrap_or_default();
        assert!(message.contains("max 2"));
    }

    #[test]
    fn validate_character_class_skipped() {
        // `[(?{]` should not trigger embedded code detection in validate()
        let v = RegexValidator::new();
        assert!(v.validate("[(?{]", 0).is_ok());
    }

    #[test]
    fn validate_rejects_embedded_code() {
        let v = RegexValidator::new();
        assert!(v.validate("(?{ print 'hi' })", 0).is_err());
        assert!(v.validate("(??{ some_code() })", 0).is_err());
    }

    #[test]
    fn validate_rejects_nested_quantifiers() {
        let v = RegexValidator::new();
        assert!(v.validate("(a+)+", 0).is_err());
        assert!(v.validate("(a+){2,5}", 0).is_err());
    }

    #[test]
    fn validate_allows_escaped_and_char_class_quantifiers() {
        let v = RegexValidator::new();
        assert!(v.validate(r"(a\+)+", 0).is_ok());
        assert!(v.validate("([a+])+", 0).is_ok());
        assert!(v.validate("([a{2}])+", 0).is_ok());
    }

    // --- RegexValidator::detects_code_execution ---

    #[test]
    fn detects_code_execution_with_code_block() {
        let v = RegexValidator::new();
        assert!(v.detects_code_execution("(?{ print 'hi' })"));
    }

    #[test]
    fn detects_code_execution_with_deferred_code_block() {
        let v = RegexValidator::new();
        assert!(v.detects_code_execution("(??{ some_code() })"));
    }

    #[test]
    fn detects_code_execution_false_for_non_capturing() {
        let v = RegexValidator::new();
        assert!(!v.detects_code_execution("(?:foo)"));
        assert!(!v.detects_code_execution("(?=ahead)"));
        assert!(!v.detects_code_execution("(?!not)"));
    }

    #[test]
    fn detects_code_execution_escaped_paren_not_detected() {
        let v = RegexValidator::new();
        assert!(!v.detects_code_execution(r"\(?{"));
    }

    #[test]
    fn detects_code_execution_in_char_class_not_detected() {
        let v = RegexValidator::new();
        assert!(!v.detects_code_execution("[(?{]"));
    }

    #[test]
    fn detects_code_execution_empty_pattern() {
        let v = RegexValidator::new();
        assert!(!v.detects_code_execution(""));
    }

    // --- RegexValidator::detect_nested_quantifiers ---

    #[test]
    fn detect_nested_quantifiers_finds_plus_plus() {
        let v = RegexValidator::new();
        assert!(v.detect_nested_quantifiers("(a+)+"));
    }

    #[test]
    fn detect_nested_quantifiers_finds_star_star() {
        let v = RegexValidator::new();
        assert!(v.detect_nested_quantifiers("(a*)*"));
    }

    #[test]
    fn detect_nested_quantifiers_finds_brace_quantifier() {
        let v = RegexValidator::new();
        assert!(v.detect_nested_quantifiers("(a+){2,5}"));
    }

    #[test]
    fn detect_nested_quantifiers_safe_patterns() {
        let v = RegexValidator::new();
        assert!(!v.detect_nested_quantifiers("(abc)+")); // no inner quantifier
        assert!(!v.detect_nested_quantifiers("[a-z]+")); // character class, not group
        assert!(!v.detect_nested_quantifiers("a+b+")); // quantifiers outside groups
    }

    // --- RegexValidator::Default ---

    #[test]
    fn default_is_same_as_new() {
        let v: RegexValidator = Default::default();
        assert!(v.validate("simple", 0).is_ok());
    }

    // --- RegexAnalyzer::extract_named_captures ---

    #[test]
    fn extract_named_captures_angle_bracket_syntax() {
        let caps = RegexAnalyzer::extract_named_captures(r"(?<year>\d{4})-(?<month>\d{2})");
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0].name, "year");
        assert_eq!(caps[0].index, 1);
        assert_eq!(caps[1].name, "month");
        assert_eq!(caps[1].index, 2);
    }

    #[test]
    fn extract_named_captures_single_quote_syntax() {
        let caps = RegexAnalyzer::extract_named_captures(r"(?'name'\w+)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "name");
        assert_eq!(caps[0].index, 1);
    }

    #[test]
    fn extract_named_captures_no_captures() {
        let caps = RegexAnalyzer::extract_named_captures(r"\d+\.\d+");
        assert!(caps.is_empty());
    }

    #[test]
    fn extract_named_captures_non_capturing_group_not_counted() {
        let caps = RegexAnalyzer::extract_named_captures(r"(?:foo)(?<bar>baz)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "bar");
        assert_eq!(caps[0].index, 1); // plain capturing groups before it still count
    }

    #[test]
    fn extract_named_captures_lookbehind_not_counted() {
        // (?<= ...) is lookbehind, not a named capture
        let caps = RegexAnalyzer::extract_named_captures(r"(?<=foo)(?<word>\w+)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "word");
    }

    #[test]
    fn extract_named_captures_escaped_paren_skipped() {
        let caps = RegexAnalyzer::extract_named_captures(r"\((?<x>\d)\)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].name, "x");
    }

    #[test]
    fn extract_named_captures_stores_subpattern() {
        let caps = RegexAnalyzer::extract_named_captures(r"(?<id>\d+)");
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].pattern, r"\d+");
    }

    // --- RegexAnalyzer::hover_text_for_regex ---

    #[test]
    fn hover_text_includes_pattern_and_captures() {
        let text = RegexAnalyzer::hover_text_for_regex(r"(?<id>\d+)", "i");
        assert!(text.contains("id"));
        assert!(text.contains("case"));
    }

    #[test]
    fn hover_text_modifier_explanations() {
        let text = RegexAnalyzer::hover_text_for_regex("foo", "imsx");
        assert!(text.contains("case-insensitive"));
        assert!(text.contains("multiline"));
        assert!(text.contains("single-line"));
        assert!(text.contains("extended"));
    }

    #[test]
    fn hover_text_global_modifier() {
        let text = RegexAnalyzer::hover_text_for_regex("foo", "g");
        assert!(text.contains("global"));
    }

    #[test]
    fn hover_text_no_modifiers() {
        let text = RegexAnalyzer::hover_text_for_regex("hello", "");
        assert!(text.contains("hello"));
        assert!(!text.contains("Modifiers"));
    }

    #[test]
    fn hover_text_empty_pattern() {
        let text = RegexAnalyzer::hover_text_for_regex("", "");
        assert!(text.is_empty());
    }

    #[test]
    fn hover_text_unknown_modifier_ignored() {
        let text = RegexAnalyzer::hover_text_for_regex("x", "z");
        // z is not a known modifier, so no modifier section
        assert!(!text.contains("Modifiers"));
    }
}
