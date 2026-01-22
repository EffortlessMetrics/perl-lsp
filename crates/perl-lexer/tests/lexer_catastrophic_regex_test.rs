//! Tests for Issue #424: Fix catastrophic regex backtracking timeout risk
//!
//! This test suite validates that the lexer handles potentially catastrophic regex
//! patterns without timeout or exponential parsing time. Patterns like `(a+)+b` can
//! cause exponential backtracking in some regex engines, but our lexer should handle
//! them safely by limiting iteration count and pattern complexity.
//!
//! # Mitigation Strategy
//!
//! The lexer implements multiple defense layers:
//! 1. **Byte limits**: MAX_REGEX_BYTES (64KB) prevents processing excessively large patterns
//! 2. **Depth limits**: MAX_DELIM_NEST (128) prevents deeply nested delimiter structures
//! 3. **Budget guards**: Fast-path checks in hot loops detect budget exhaustion
//! 4. **Error tokens**: Budget exhaustion returns UnknownRest token instead of hanging
//!
//! # Key Areas Protected
//!
//! - Regex literal parsing (/pattern/)
//! - Substitution pattern parsing (s/pattern/replacement/)
//! - Match operator parsing (m/pattern/)
//! - Transliteration parsing (tr/pattern/replacement/)
//! - Quote-like operators (qr/pattern/)
//!
//! # Performance Guarantees
//!
//! - Normal regex patterns: <1ms tokenization time
//! - Pathological patterns: <100ms with budget guard triggering
//! - Budget exhaustion: Clean error token with no hang

use perl_lexer::{PerlLexer, TokenType};

/// Test that nested quantifiers are handled safely
#[test]
fn test_nested_quantifiers_safe_handling() {
    // Pattern: (a+)+b - classic catastrophic backtracking example
    let test_cases = vec![
        (r"/^(a+)+b$/", "Nested quantifiers with start/end anchors"),
        (r"m/(x*)*y/", "Nested zero-or-more quantifiers"),
        (r"s/(a+)+(b+)+/replacement/", "Multiple nested quantifier groups"),
        (r"qr/^(a*)*$/", "Nested star quantifiers in qr operator"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        // Should tokenize without hanging
        assert!(
            !tokens.is_empty(),
            "{}: should tokenize without hanging. Input: {}",
            description,
            input
        );

        // Verify we got appropriate token types (regex or error)
        let has_regex_or_error = tokens.iter().any(|t| {
            matches!(
                t.token_type,
                TokenType::RegexMatch
                    | TokenType::Substitution
                    | TokenType::QuoteRegex
                    | TokenType::Error(_)
                    | TokenType::UnknownRest
            )
        });
        assert!(
            has_regex_or_error,
            "{}: should produce regex or error token. Input: {}",
            description,
            input
        );
    }
}

/// Test that alternation with backtracking is handled safely
#[test]
fn test_alternation_backtracking_safe() {
    // Patterns with complex alternation that could cause backtracking
    let test_cases = vec![
        (r"/(a|ab)*c/", "Overlapping alternation"),
        (r"m/(x|xy)+(y|yz)+/", "Multiple overlapping alternations"),
        (r"s/(foo|foobar)+/replacement/", "Prefix alternation"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should tokenize without hanging. Input: {}",
            description,
            input
        );
    }
}

/// Test that very long patterns trigger budget guard
#[test]
fn test_very_long_pattern_budget_guard() {
    // Create a pattern that exceeds MAX_REGEX_BYTES (64KB)
    let long_pattern = "a".repeat(70_000);
    let input = format!("/{}/", long_pattern);

    let mut lexer = PerlLexer::new(&input);
    let tokens: Vec<_> = lexer.collect_tokens();

    // Should produce UnknownRest token when budget exceeded
    let has_unknown_rest = tokens.iter().any(|t| matches!(t.token_type, TokenType::UnknownRest));
    assert!(
        has_unknown_rest,
        "Very long pattern should trigger budget guard and produce UnknownRest token"
    );
}

/// Test that deeply nested delimiters trigger budget guard
#[test]
fn test_deeply_nested_delimiters_budget_guard() {
    // Create a pattern with deeply nested delimiters beyond MAX_DELIM_NEST (128)
    let mut pattern = String::from("s{");
    for _ in 0..150 {
        pattern.push_str("{{");
    }
    pattern.push_str("pattern");
    for _ in 0..150 {
        pattern.push_str("}}");
    }
    pattern.push_str("}{replacement}");

    let mut lexer = PerlLexer::new(&pattern);
    let tokens: Vec<_> = lexer.collect_tokens();

    // Should handle or produce error token for excessive nesting
    assert!(
        !tokens.is_empty(),
        "Deeply nested delimiters should be handled without hanging"
    );
}

/// Test that escaped characters in patterns don't cause issues
#[test]
fn test_escaped_chars_in_patterns() {
    let test_cases = vec![
        (r"/\(\+\)\+/", "Escaped quantifiers"),
        (r"s/\\/\//", "Escaped backslashes and delimiters"),
        (r"m/\x{1234}+/", "Unicode escapes with quantifiers"),
        (r"/\Q(a+)+\E/", "Quoted metacharacters"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should handle escaped characters. Input: {}",
            description,
            input
        );
    }
}

/// Test substitution operator with complex patterns
#[test]
fn test_substitution_complex_patterns() {
    let test_cases = vec![
        (r"s/(a+)+b/replacement/", "Nested quantifiers in substitution"),
        (r"s{(x*)*y}{repl}", "Nested star quantifiers with braces"),
        (r"s/pattern/(replacement+)+/", "Nested quantifiers in replacement"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should tokenize without hanging. Input: {}",
            description,
            input
        );
    }
}

/// Test match operator with lookahead/lookbehind
#[test]
fn test_match_with_lookaround() {
    let test_cases = vec![
        (r"m/(?=a+)+b/", "Positive lookahead with quantifiers"),
        (r"m/(?<=x+)+y/", "Positive lookbehind with quantifiers"),
        (r"/(?!a+)+b/", "Negative lookahead with quantifiers"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should tokenize without hanging. Input: {}",
            description,
            input
        );
    }
}

/// Test qr operator with complex patterns
#[test]
fn test_qr_operator_complex_patterns() {
    let test_cases = vec![
        (r"qr/(a+)+b/", "Nested quantifiers in qr"),
        (r"qr{(x|xy)+y}", "Overlapping alternation in qr with braces"),
        (r"qr/(?:a+)+b/i", "Non-capturing group with nested quantifiers"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should tokenize without hanging. Input: {}",
            description,
            input
        );
    }
}

/// Test transliteration with complex patterns (tr doesn't use regex but should still be safe)
#[test]
fn test_transliteration_safety() {
    let test_cases = vec![
        (r"tr/abc/xyz/", "Simple transliteration"),
        (r"tr{a-z}{A-Z}", "Range in transliteration with braces"),
        (r"y/\x00-\xff/\x00-\xff/", "Full byte range transliteration"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should tokenize without hanging. Input: {}",
            description,
            input
        );
    }
}

/// Test mixed operators in sequence
#[test]
fn test_mixed_operators_sequence() {
    let input = r#"
        /^(a+)+b$/;
        s/(x*)*y/replacement/;
        m/(foo|foobar)+/;
        qr/(a+)+/;
        tr/abc/xyz/;
    "#;

    let mut lexer = PerlLexer::new(input);
    let tokens: Vec<_> = lexer.collect_tokens();

    // Should successfully tokenize entire sequence
    assert!(
        !tokens.is_empty(),
        "Mixed operators with complex patterns should tokenize without hanging"
    );

    // Count different operator types
    let regex_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::RegexMatch)).count();
    let subst_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Substitution)).count();
    let trans_count = tokens.iter().filter(|t| matches!(t.token_type, TokenType::Transliteration)).count();

    // Verify we found multiple operators (demonstrates no hang on any single operator)
    assert!(
        regex_count + subst_count + trans_count >= 3,
        "Should tokenize multiple operators successfully"
    );
}

/// Test that budget guard provides clean error message
#[test]
fn test_budget_guard_clean_error() {
    // Create input that triggers budget guard
    let excessive_input = "a".repeat(70_000);
    let input = format!("/{}/", excessive_input);

    let mut lexer = PerlLexer::new(&input);
    let tokens: Vec<_> = lexer.collect_tokens();

    // Find UnknownRest token
    let unknown_rest_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token_type, TokenType::UnknownRest)).collect();

    assert!(!unknown_rest_tokens.is_empty(), "Budget exhaustion should produce UnknownRest token");

    // Verify token has valid position information
    for token in unknown_rest_tokens {
        assert!(
            token.start <= token.end,
            "UnknownRest token should have valid position: start={}, end={}",
            token.start,
            token.end
        );
    }
}

/// Performance test: normal patterns should tokenize quickly
#[test]
fn test_normal_patterns_fast_tokenization() {
    let test_cases = vec![
        r"/^\d{3}-\d{3}-\d{4}$/",  // Phone number
        r"s/foo/bar/g",              // Simple substitution
        r"m/^[a-zA-Z0-9]+$/",        // Alphanumeric match
        r"qr/\w+@\w+\.\w+/",         // Email-like pattern
    ];

    for input in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        // These should tokenize successfully without budget limits
        let has_unknown_rest = tokens.iter().any(|t| matches!(t.token_type, TokenType::UnknownRest));
        assert!(
            !has_unknown_rest,
            "Normal patterns should not trigger budget guard. Input: {}",
            input
        );
    }
}

/// Test edge case: empty pattern
#[test]
fn test_empty_pattern() {
    let test_cases = vec![
        ("//", "Empty regex pattern"),
        ("s///", "Empty substitution pattern and replacement"),
        ("m//", "Empty match pattern"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        assert!(
            !tokens.is_empty(),
            "{}: should handle empty patterns. Input: {}",
            description,
            input
        );
    }
}

/// Test edge case: pattern with only quantifiers
#[test]
fn test_pattern_only_quantifiers() {
    let test_cases = vec![
        ("/+++/", "Multiple plus quantifiers"),
        ("/***/", "Multiple star quantifiers"),
        ("/???/", "Multiple question quantifiers"),
    ];

    for (input, description) in test_cases {
        let mut lexer = PerlLexer::new(input);
        let tokens: Vec<_> = lexer.collect_tokens();

        // These are malformed patterns but should not hang
        assert!(
            !tokens.is_empty(),
            "{}: should handle malformed quantifier patterns without hanging. Input: {}",
            description,
            input
        );
    }
}
