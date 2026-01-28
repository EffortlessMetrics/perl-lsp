/// Tests for regex with arbitrary delimiters (Issue #444)
/// Verifies that the lexer correctly tokenizes m, s, qr, tr/y operators with non-slash delimiters
use perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_m_operator_exclamation_delimiter() {
    let code = r#"m!pattern!"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    // Should have RegexMatch token and EOF
    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::RegexMatch),
        "Expected RegexMatch token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "m!pattern!");
}

#[test]
fn test_m_operator_brace_delimiter() {
    let code = r#"m{pattern}"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::RegexMatch),
        "Expected RegexMatch token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "m{pattern}");
}

#[test]
fn test_m_operator_pipe_delimiter() {
    let code = r#"m|pattern|"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::RegexMatch),
        "Expected RegexMatch token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "m|pattern|");
}

#[test]
fn test_m_operator_hash_delimiter() {
    let code = r#"m#pattern#"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::RegexMatch),
        "Expected RegexMatch token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "m#pattern#");
}

#[test]
fn test_s_operator_exclamation_delimiter() {
    let code = r#"s!old!new!"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::Substitution),
        "Expected Substitution token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "s!old!new!");
}

#[test]
fn test_s_operator_brace_delimiter() {
    let code = r#"s{old}{new}"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::Substitution),
        "Expected Substitution token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "s{old}{new}");
}

#[test]
fn test_s_operator_pipe_delimiter() {
    let code = r#"s|old|new|"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::Substitution),
        "Expected Substitution token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "s|old|new|");
}

#[test]
fn test_qr_operator_exclamation_delimiter() {
    let code = r#"qr!pattern!i"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::QuoteRegex),
        "Expected QuoteRegex token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "qr!pattern!i");
}

#[test]
fn test_tr_operator_exclamation_delimiter() {
    let code = r#"tr!abc!xyz!"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::Transliteration),
        "Expected Transliteration token, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "tr!abc!xyz!");
}

#[test]
fn test_y_operator_pipe_delimiter() {
    let code = r#"y|abc|xyz|"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::Transliteration),
        "Expected Transliteration token for y alias, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "y|abc|xyz|");
}

#[test]
fn test_nested_braces_in_m_operator() {
    let code = r#"m{pattern{nested}}"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens");
    assert!(
        matches!(tokens[0].token_type, TokenType::RegexMatch),
        "Expected RegexMatch token with nested braces, got: {:?}",
        tokens[0].token_type
    );
    assert_eq!(tokens[0].text.as_ref(), "m{pattern{nested}}");
}

#[test]
fn test_modifiers_attached_to_m_operator() {
    let code = r#"m!pattern!imsxgc"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens (regex + EOF)");
    assert!(
        matches!(tokens[0].token_type, TokenType::RegexMatch),
        "Expected RegexMatch token, got: {:?}",
        tokens[0].token_type
    );
    // Modifiers should be part of the regex token
    assert_eq!(tokens[0].text.as_ref(), "m!pattern!imsxgc");
}

#[test]
fn test_modifiers_attached_to_s_operator() {
    let code = r#"s|old|new|ge"#;
    let mut lexer = PerlLexer::new(code);
    let tokens = lexer.collect_tokens();

    assert_eq!(tokens.len(), 2, "Expected 2 tokens (subst + EOF)");
    assert!(
        matches!(tokens[0].token_type, TokenType::Substitution),
        "Expected Substitution token, got: {:?}",
        tokens[0].token_type
    );
    // Modifiers should be part of the substitution token
    assert_eq!(tokens[0].text.as_ref(), "s|old|new|ge");
}

#[test]
fn test_various_non_standard_delimiters() {
    let test_cases = vec![
        ("m~pattern~", TokenType::RegexMatch, "tilde"),
        ("m@pattern@", TokenType::RegexMatch, "at"),
        ("m%pattern%", TokenType::RegexMatch, "percent"),
        ("m^pattern^", TokenType::RegexMatch, "caret"),
        ("m&pattern&", TokenType::RegexMatch, "ampersand"),
        ("m*pattern*", TokenType::RegexMatch, "asterisk"),
        ("m-pattern-", TokenType::RegexMatch, "dash"),
        ("m+pattern+", TokenType::RegexMatch, "plus"),
        ("m=pattern=", TokenType::RegexMatch, "equals"),
        ("m:pattern:", TokenType::RegexMatch, "colon"),
        ("m;pattern;", TokenType::RegexMatch, "semicolon"),
        ("m,pattern,", TokenType::RegexMatch, "comma"),
        ("m.pattern.", TokenType::RegexMatch, "dot"),
    ];

    for (code, expected_type, delim_name) in test_cases {
        let mut lexer = PerlLexer::new(code);
        let tokens = lexer.collect_tokens();

        assert_eq!(tokens.len(), 2, "Expected 2 tokens for {}", delim_name);
        assert!(
            tokens[0].token_type == expected_type,
            "Failed to parse m operator with {} delimiter: {:?}",
            delim_name,
            tokens[0].token_type
        );
        assert_eq!(tokens[0].text.as_ref(), code, "Text mismatch for {}", delim_name);
    }
}

#[test]
fn test_m_vs_bareword_disambiguation() {
    // When followed by valid delimiter, should be regex
    let code1 = r#"m!pattern!"#;
    let mut lexer1 = PerlLexer::new(code1);
    let tokens1 = lexer1.collect_tokens();
    assert!(
        matches!(tokens1[0].token_type, TokenType::RegexMatch),
        "m followed by ! should be regex match"
    );

    // When followed by whitespace and then delimiter, should still work for m
    let code2 = "m (pattern)";  // Note: for m, delimiter must be adjacent, so this might not work
    let mut lexer2 = PerlLexer::new(code2);
    let tokens2 = lexer2.collect_tokens();
    // For m operator, delimiter must be immediately adjacent, so this should be identifier + paren
    // This test verifies the correct behavior
    assert!(
        matches!(tokens2[0].token_type, TokenType::Identifier(_)),
        "m followed by space should be identifier, got: {:?}",
        tokens2[0].token_type
    );
}
