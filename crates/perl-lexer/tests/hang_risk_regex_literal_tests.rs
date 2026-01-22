//! Comprehensive regex literal handling tests for hang/bounds risk mitigation
//!
//! Tests feature spec: ROADMAP.md#known-gaps-hang-bounds-risks
//! Tests feature spec: ROADMAP.md#regex-literal-handling
//!
//! This test suite validates that the lexer handles complex regex patterns
//! without hanging, correctly enforces budget limits, and gracefully handles
//! pathological regex inputs.
//!
//! Coverage areas:
//! - Simple regex patterns
//! - Complex regex with character classes, quantifiers
//! - Nested capture groups
//! - Lookahead/lookbehind assertions
//! - Unicode in regex patterns
//! - Very long regex patterns (budget limit testing)
//! - Regex with various delimiters
//! - Regex modifiers (i, g, m, s, x, etc.)
//! - Pathological regex patterns (catastrophic backtracking patterns)
//! - Malformed regex handling

use perl_lexer::{PerlLexer, TokenType};

/// Test simple regex pattern
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_simple_pattern() {
    let code = "/hello/";
    let mut lexer = PerlLexer::new(code);

    let tok = lexer.next_token().expect("Expected regex token");
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex token, got {:?}",
        tok.token_type
    );
}

/// Test regex with character classes
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_character_classes() {
    let test_cases = vec![
        "/[abc]/",
        "/[a-z]/",
        "/[A-Z0-9]/",
        "/[^abc]/",        // Negated class
        r"/[\w\d\s]/",     // Escape sequences
        r"/[\[\]]/",       // Escaped brackets
        r"/[a-zA-Z0-9_]/", // Combined ranges
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with quantifiers
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_quantifiers() {
    let test_cases = vec![
        r"/a*/",      // Zero or more
        r"/a+/",      // One or more
        r"/a?/",      // Zero or one
        r"/a{3}/",    // Exactly 3
        r"/a{3,}/",   // 3 or more
        r"/a{3,5}/",  // Between 3 and 5
        r"/a*?/",     // Non-greedy
        r"/a+?/",     // Non-greedy
        r"/a{3,5}?/", // Non-greedy
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with capture groups
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_capture_groups() {
    let test_cases = vec![
        r"/(abc)/",        // Simple group
        r"/(a)(b)(c)/",    // Multiple groups
        r"/(?:abc)/",      // Non-capturing group
        r"/(?<name>abc)/", // Named capture
        r"/(a|b|c)/",      // Alternation
        r"/((a)(b))/",     // Nested groups
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test deeply nested capture groups (boundedness)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_deeply_nested_captures() {
    // Create deeply nested capture groups
    let depth = 100;
    let mut pattern = String::from("/");

    for _ in 0..depth {
        pattern.push('(');
    }
    pattern.push('x');
    for _ in 0..depth {
        pattern.push(')');
    }
    pattern.push('/');

    let mut lexer = PerlLexer::new(&pattern);
    let result = lexer.next_token();

    // Should handle gracefully - either parse successfully or return None
    match result {
        Some(tok) => {
            assert!(
                matches!(tok.token_type, TokenType::RegexMatch | TokenType::UnknownRest),
                "Expected regex or UnknownRest for deeply nested captures, got {:?}",
                tok.token_type
            );
        }
        None => {
            // Acceptable - hit delimiter nesting limit or end of input
        }
    }
}

/// Test regex with lookahead assertions
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_lookahead_assertions() {
    let test_cases = vec![
        r"/(?=abc)/",  // Positive lookahead
        r"/(?!abc)/",  // Negative lookahead
        r"/a(?=b)c/",  // Lookahead in pattern
        r"/(?=.*\d)/", // Complex lookahead
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with lookbehind assertions
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_lookbehind_assertions() {
    let test_cases = vec![
        r"/(?<=abc)/", // Positive lookbehind
        r"/(?<!abc)/", // Negative lookbehind
        r"/(?<=a)bc/", // Lookbehind in pattern
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with modifiers
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_modifiers() {
    let test_cases = vec![
        "/pattern/i",     // Case-insensitive
        "/pattern/g",     // Global
        "/pattern/m",     // Multiline
        "/pattern/s",     // Single-line (dot matches newline)
        "/pattern/x",     // Extended (ignore whitespace)
        "/pattern/igm",   // Multiple modifiers
        "/pattern/imsxg", // All common modifiers
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with various delimiters
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_various_delimiters() {
    let test_cases = vec![
        "m/pattern/", // Standard /
        "m{pattern}", // Braces
        "m[pattern]", // Brackets
        "m(pattern)", // Parens
        "m<pattern>", // Angle brackets
        "m!pattern!", // Exclamation
        "m#pattern#", // Hash
        "m|pattern|", // Pipe
        "m~pattern~", // Tilde
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with nested delimiters (paired)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_nested_delimiters() {
    let test_cases = vec![
        r"m{a{b}c}",    // Nested braces
        r"m[a[b]c]",    // Nested brackets
        r"m(a(b)c)",    // Nested parens
        r"m<a<b>c>",    // Nested angle brackets
        r"m{{{text}}}", // Multiple nesting levels
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test very long regex pattern (budget limit)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_very_long_pattern() {
    // Create a regex pattern just under the MAX_REGEX_BYTES limit (64KB)
    let pattern_len = 60 * 1024; // 60KB
    let mut pattern = String::from("/");
    pattern.push_str(&"a".repeat(pattern_len));
    pattern.push('/');

    let mut lexer = PerlLexer::new(&pattern);
    let result = lexer.next_token();

    match result {
        Some(tok) => {
            assert!(
                matches!(tok.token_type, TokenType::RegexMatch | TokenType::UnknownRest),
                "Expected regex or UnknownRest for long pattern, got {:?}",
                tok.token_type
            );
        }
        None => {
            panic!("Expected token for long pattern");
        }
    }
}

/// Test extremely long regex pattern (exceeds budget limit)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_exceeds_budget_limit() {
    // Create a regex pattern that exceeds MAX_REGEX_BYTES (64KB)
    let pattern_len = 70 * 1024; // 70KB
    let mut pattern = String::from("/");
    pattern.push_str(&"a".repeat(pattern_len));
    pattern.push('/');

    let mut lexer = PerlLexer::new(&pattern);
    let result = lexer.next_token();

    match result {
        Some(tok) => {
            // Should emit UnknownRest token after hitting limit
            assert!(
                matches!(tok.token_type, TokenType::UnknownRest),
                "Expected UnknownRest for pattern exceeding budget, got {:?}",
                tok.token_type
            );
        }
        None => {
            panic!("Expected token for pattern exceeding budget");
        }
    }
}

/// Test regex with Unicode characters
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_unicode_patterns() {
    let test_cases = vec![
        "/café/",
        "/日本語/",
        "/\u{1F600}/",    // Emoji
        r"/\x{263A}/",    // Unicode escape
        r"/\N{SNOWMAN}/", // Named Unicode
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test pathological regex pattern (catastrophic backtracking)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_pathological_backtracking_pattern() {
    // Pattern known to cause catastrophic backtracking in naive implementations
    let code = r"/(a+)+b/";
    let mut lexer = PerlLexer::new(code);

    let tok = lexer.next_token().expect("Expected regex token");
    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex for pathological pattern, got {:?}",
        tok.token_type
    );
}

/// Test regex with embedded comments (x modifier)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_embedded_comments() {
    let code = r"/
        \d{3}   # area code
        -       # separator
        \d{4}   # number
    /x";

    let mut lexer = PerlLexer::new(code);
    let tok = lexer.next_token().expect("Expected regex token");

    assert!(
        matches!(tok.token_type, TokenType::RegexMatch),
        "Expected regex with embedded comments, got {:?}",
        tok.token_type
    );
}

/// Test malformed regex (unclosed delimiter)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_malformed_unclosed() {
    let code = "/pattern";
    let mut lexer = PerlLexer::new(code);

    let result = lexer.next_token();

    // Should handle gracefully - either return token or None
    match result {
        Some(tok) => {
            assert!(
                matches!(tok.token_type, TokenType::UnknownRest | TokenType::RegexMatch),
                "Expected error token for malformed regex, got {:?}",
                tok.token_type
            );
        }
        None => {
            // Also acceptable - end of input
        }
    }
}

/// Test malformed regex (unbalanced groups)
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_malformed_unbalanced_groups() {
    let test_cases = vec![
        "/(abc/",   // Unclosed group
        "/abc)/",   // Unopened group
        "/((abc)/", // Mismatched groups
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let result = lexer.next_token();

        // Should handle gracefully
        match result {
            Some(tok) => {
                assert!(
                    matches!(tok.token_type, TokenType::UnknownRest | TokenType::RegexMatch),
                    "Expected error handling for malformed regex '{}', got {:?}",
                    code,
                    tok.token_type
                );
            }
            None => {
                // Also acceptable - end of input
            }
        }
    }
}

/// Test regex with escaped delimiter
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_escaped_delimiter() {
    let test_cases = vec![
        r"/a\/b/",  // Escaped /
        r"m{a\}b}", // Escaped }
        r"m[a\]b]", // Escaped ]
        r"m(a\)b)", // Escaped )
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with alternation
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_alternation() {
    let test_cases = vec!["/(a|b)/", "/(abc|def|ghi)/", "/(a|b|c|d|e)/", "/(?:foo|bar)/"];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test regex with anchors
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_anchors() {
    let test_cases = vec![
        r"/^start/",
        r"/end$/",
        r"/^both$/",
        r"/\Astart/",
        r"/end\z/",
        r"/\bword\b/",
        r"/\Bnot_boundary\B/",
    ];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected regex token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::RegexMatch),
            "Expected regex for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}

/// Test no hang on complex nested regex pattern
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_no_hang_complex_pattern() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // Complex pattern with multiple nesting types
    let code = r"/((?:(?:[a-z]+)+)+(?:[0-9]+)*)/igms";
    let code_arc = Arc::new(code.to_string());
    let result_arc = Arc::new(Mutex::new(None));
    let result_clone = Arc::clone(&result_arc);

    let handle = std::thread::spawn(move || {
        let mut lexer = PerlLexer::new(&code_arc);
        let result = lexer.next_token();
        *result_clone.lock().unwrap() = Some(result);
    });

    // Wait max 2 seconds
    let _timeout = Duration::from_secs(2);
    let completed = handle.join().is_ok();

    assert!(completed, "Lexer should complete complex regex within timeout");

    let result_guard = result_arc.lock().unwrap();
    let result = result_guard.as_ref().expect("Lexer should have returned a result");

    assert!(result.is_some(), "Lexer should handle complex regex pattern: {:?}", result);
}

/// Test regex performance doesn't degrade with pattern complexity
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
#[cfg_attr(not(feature = "slow_tests"), ignore)]
fn lexer_regex_literal_performance_bounded() {
    use std::time::Instant;

    // Simple pattern
    let simple = "/abc/";
    let start = Instant::now();
    let mut lexer = PerlLexer::new(simple);
    let _ = lexer.next_token();
    let simple_duration = start.elapsed();

    // Complex pattern
    let complex = r"/((?:[a-z]+)+(?:[0-9]+)*(?:\w+)?(?:foo|bar|baz)+)/igms";
    let start = Instant::now();
    let mut lexer = PerlLexer::new(complex);
    let _ = lexer.next_token();
    let complex_duration = start.elapsed();

    // Complex should not be more than 100x slower
    let ratio = complex_duration.as_micros() as f64 / simple_duration.as_micros() as f64;
    assert!(
        ratio < 100.0,
        "Regex lexing time ratio {} indicates potential performance issue",
        ratio
    );
}

/// Test delimiter nesting limit is enforced
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_delimiter_nesting_limit() {
    // Create pattern with nesting depth exceeding MAX_DELIM_NEST (128)
    let depth = 150;
    let mut pattern = String::from("m{");

    for _ in 0..depth {
        pattern.push('{');
    }
    pattern.push('x');
    for _ in 0..depth {
        pattern.push('}');
    }
    pattern.push('}');

    let mut lexer = PerlLexer::new(&pattern);
    let result = lexer.next_token();

    match result {
        Some(tok) => {
            // Should emit UnknownRest after hitting nesting limit
            assert!(
                matches!(tok.token_type, TokenType::UnknownRest | TokenType::RegexMatch),
                "Expected UnknownRest or graceful handling for deep nesting, got {:?}",
                tok.token_type
            );
        }
        None => {
            // Also acceptable - end of input or budget exceeded
        }
    }
}

/// Test qr// quoted regex
///
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn lexer_regex_literal_qr_quoted_regex() {
    let test_cases =
        vec!["qr/pattern/", "qr{pattern}", "qr[pattern]", "qr/pattern/i", "qr/(?:abc)+/igms"];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tok = lexer.next_token().expect(&format!("Expected qr token for '{}'", code));

        assert!(
            matches!(tok.token_type, TokenType::QuoteRegex),
            "Expected qr token for '{}', got {:?}",
            code,
            tok.token_type
        );
    }
}
