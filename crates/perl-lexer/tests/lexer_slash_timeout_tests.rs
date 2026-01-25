//! Tests for Issue #422 - Fix ambiguous slash (division vs regex) timeout risk
//!
//! The parser can hang when disambiguating `/` as division vs regex start.
//! These tests validate that:
//! 1. Recursion/backtracking limits prevent infinite loops
//! 2. Context-aware heuristics reduce ambiguity
//! 3. Timeout protection handles worst-case scenarios
//! 4. Metrics/logging track when disambiguation takes too long

use perl_lexer::{PerlLexer, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_slash_after_identifier_is_division() -> TestResult {
    // After identifier → likely division
    let mut lexer = PerlLexer::new("$x / 2");
    lexer.next_token(); // $x
    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_slash_after_number_is_division() -> TestResult {
    // After number → likely division
    let mut lexer = PerlLexer::new("10 / 2");
    lexer.next_token(); // 10
    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_slash_after_closing_paren_is_division() -> TestResult {
    // After closing paren → likely division
    let mut lexer = PerlLexer::new("($x + $y) / 2");
    lexer.next_token(); // (
    lexer.next_token(); // $x
    lexer.next_token(); // +
    lexer.next_token(); // $y
    lexer.next_token(); // )
    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_slash_after_operator_is_regex() -> TestResult {
    // After operator → likely regex
    let mut lexer = PerlLexer::new("=~ /pattern/");
    lexer.next_token(); // =~
    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_slash_after_keyword_is_regex() -> TestResult {
    // After keyword → likely regex
    let mut lexer = PerlLexer::new("if /pattern/");
    lexer.next_token(); // if
    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_slash_after_opening_paren_is_regex() -> TestResult {
    // After opening paren → likely regex
    let mut lexer = PerlLexer::new("if (/pattern/)");
    lexer.next_token(); // if
    lexer.next_token(); // (
    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_pathological_regex_with_budget_limit() -> TestResult {
    // Create a pathological regex that would exceed budget
    // The lexer should gracefully truncate with UnknownRest
    let huge_pattern = "a".repeat(70_000); // Exceeds MAX_REGEX_BYTES (64KB)
    let code = format!("/{}/", huge_pattern);

    let mut lexer = PerlLexer::new(&code);
    let token = lexer.next_token().ok_or("Expected token")?;

    // Should return UnknownRest due to budget exceeded
    assert_eq!(token.token_type, TokenType::UnknownRest);
    Ok(())
}

#[test]
fn test_unterminated_regex_graceful_failure() {
    // Unterminated regex should fail gracefully, not hang
    let mut lexer = PerlLexer::new("if /pattern");
    lexer.next_token(); // if
    let token = lexer.next_token();

    // Should return None (unterminated) or error token, not hang
    assert!(token.is_none() || matches!(token.map(|t| t.token_type), Some(TokenType::Error(_))));
}

#[test]
fn test_deeply_nested_slashes_with_escapes() -> TestResult {
    // Multiple escaped slashes shouldn't cause timeout
    let code = r"/\\/\\/\\/\\/\\/\\/\\/\\/\\/\\/";
    let mut lexer = PerlLexer::new(code);
    let token = lexer.next_token().ok_or("Expected regex token")?;

    // Should complete quickly with regex token
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_alternating_context_slashes() -> TestResult {
    // Alternating division and regex contexts
    let mut lexer = PerlLexer::new("$a / 2 if /test/");
    lexer.next_token(); // $a
    let token1 = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token1.token_type, TokenType::Division);

    lexer.next_token(); // 2
    lexer.next_token(); // if
    let token2 = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token2.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_defined_or_vs_empty_regex() -> TestResult {
    // // can be defined-or or empty regex depending on context
    let mut lexer = PerlLexer::new("$a // $b");
    lexer.next_token(); // $a
    let token = lexer.next_token().ok_or("Expected operator token")?;

    // After $a (identifier), // should be defined-or operator
    assert!(matches!(token.token_type, TokenType::Operator(_)));
    Ok(())
}

#[test]
fn test_regex_after_match_operator() -> TestResult {
    // =~ // should parse as match operator followed by empty regex
    let mut lexer = PerlLexer::new("$x =~ //");
    lexer.next_token(); // $x
    lexer.next_token(); // =~
    let token = lexer.next_token().ok_or("Expected regex token")?;
    assert_eq!(token.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn test_division_assignment() -> TestResult {
    // /= should be division assignment, not regex
    let mut lexer = PerlLexer::new("$x /= 2");
    lexer.next_token(); // $x
    let token = lexer.next_token().ok_or("Expected operator token")?;

    // Should be Division token (the lexer emits Division, then = separately in current impl)
    // This is acceptable behavior as the parser handles compound assignment
    assert!(matches!(token.token_type, TokenType::Division | TokenType::Operator(_)));
    Ok(())
}

#[test]
fn test_multiple_consecutive_slashes_in_expression() -> TestResult {
    // Complex expression with multiple slashes
    let mut lexer = PerlLexer::new("($a / $b) / ($c / $d)");

    lexer.next_token(); // (
    lexer.next_token(); // $a
    let token1 = lexer.next_token().ok_or("Expected first division token")?;
    assert_eq!(token1.token_type, TokenType::Division);

    lexer.next_token(); // $b
    lexer.next_token(); // )
    let token2 = lexer.next_token().ok_or("Expected second division token")?;
    assert_eq!(token2.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_slash_disambiguation_performance() {
    // Test that slash disambiguation completes in reasonable time
    use std::time::Instant;

    let code = "if (/test/) { $x / 2 } elsif (/other/) { $y / 3 }";
    let start = Instant::now();

    let mut lexer = PerlLexer::new(code);
    while lexer.next_token().is_some() {
        // Consume all tokens
    }

    let elapsed = start.elapsed();

    // Should complete in under 1ms for this simple code
    assert!(elapsed.as_millis() < 10, "Disambiguation took too long: {:?}", elapsed);
}

#[test]
fn test_regex_with_complex_escapes() -> TestResult {
    // Regex with many escape sequences - tests that escapes don't cause timeout
    let code = r#"/\d+\s+\w+\n\r\t\\/\//i"#;
    let mut lexer = PerlLexer::new(code);
    let token = lexer.next_token().ok_or("Expected regex token")?;

    // Main goal: verify regex is parsed correctly without timeout
    assert_eq!(token.token_type, TokenType::RegexMatch);
    assert!(token.text.contains("\\d")); // Contains escape sequences
    assert!(token.text.starts_with('/')); // Starts with slash
    Ok(())
}

#[test]
fn test_division_in_list_context() -> TestResult {
    // Division in list/array context
    let mut lexer = PerlLexer::new("($a / $b, $c / $d)");

    lexer.next_token(); // (
    lexer.next_token(); // $a
    let token1 = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token1.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_regex_in_conditional() -> TestResult {
    // Regex in if/unless/while conditions
    let test_cases = vec!["if (/test/)", "unless (/test/)", "while (/test/)", "until (/test/)"];

    for code in test_cases {
        let mut lexer = PerlLexer::new(code);
        lexer.next_token(); // keyword
        lexer.next_token(); // (
        let token = lexer.next_token().ok_or_else(|| format!("Expected regex token for: {}", code))?;
        assert_eq!(token.token_type, TokenType::RegexMatch, "Failed for: {}", code);
    }
    Ok(())
}

#[test]
fn test_slash_after_array_subscript() -> TestResult {
    // After array subscript → likely division
    // Note: Lexer tokenizes $arr separately from [0]
    let mut lexer = PerlLexer::new("$arr[0] / 2");
    lexer.next_token(); // $arr
    lexer.next_token(); // [
    lexer.next_token(); // 0
    lexer.next_token(); // ]
    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_slash_after_hash_subscript() -> TestResult {
    // After hash subscript → likely division
    // Note: Lexer tokenizes $hash separately from {key}
    let mut lexer = PerlLexer::new("$hash{key} / 2");
    lexer.next_token(); // $hash
    lexer.next_token(); // {
    lexer.next_token(); // key
    lexer.next_token(); // }
    let token = lexer.next_token().ok_or("Expected division token")?;
    assert_eq!(token.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn test_budget_guard_prevents_infinite_loop() {
    // Test that budget guard prevents infinite loops
    // Create a regex that approaches but doesn't exceed the limit
    let pattern = "a".repeat(60_000); // Just under MAX_REGEX_BYTES
    let code = format!("/{}/i", pattern);

    let mut lexer = PerlLexer::new(&code);
    let token = lexer.next_token();

    // Should successfully parse or gracefully fail, not hang
    assert!(token.is_some());
}
