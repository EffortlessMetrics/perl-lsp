use perl_lexer::{PerlLexer, TokenType};

#[test]
fn lexer_emits_eof_once() {
    // Empty input
    let mut lx = PerlLexer::new("");
    let t1 = lx.next_token().unwrap();
    assert!(matches!(t1.token_type, TokenType::EOF), "First token should be EOF");
    assert!(lx.next_token().is_none(), "After EOF, should return None");
    
    // Whitespace only (lexer skips whitespace so should go directly to EOF)
    let mut lx = PerlLexer::new("   ");
    let t1 = lx.next_token().unwrap();
    assert!(matches!(t1.token_type, TokenType::EOF), "First token should be EOF (whitespace skipped)");
    assert!(lx.next_token().is_none(), "After EOF, should return None");
    
    // With actual token
    let mut lx = PerlLexer::new("print");
    let t1 = lx.next_token().unwrap();
    assert!(matches!(t1.token_type, TokenType::Keyword(_)), "First token should be keyword");
    let t2 = lx.next_token().unwrap();
    assert!(matches!(t2.token_type, TokenType::EOF), "Second token should be EOF");
    assert!(lx.next_token().is_none(), "After EOF, should return None");
}

#[test]
fn word_op_without_delim_is_identifier() {
    for op in ["q", "qq", "qw", "qr", "qx", "m", "s", "tr", "y"] {
        let mut lx = PerlLexer::new(op); // no delimiter
        let t = lx.next_token().unwrap();
        assert!(
            matches!(t.token_type, TokenType::Identifier(_)),
            "op={op} should be identifier, got {:?}",
            t.token_type
        );
    }
}

#[test]
fn word_op_with_space_before_delim_is_identifier() {
    // Test that quote operators with space before delimiter become identifiers
    for test in ["q ", "qq\n", "qw\t", "m   "] {
        let mut lx = PerlLexer::new(test);
        let t = lx.next_token().unwrap();
        assert!(
            matches!(t.token_type, TokenType::Identifier(_)),
            "'{test}' first token should be identifier, got {:?}",
            t.token_type
        );
    }
}

#[test]
fn quote_ops_with_delimiters_tokenize_correctly() {
    // Test that quote operators with delimiters produce correct tokens
    let tests = vec![
        ("q{test}", "QuoteSingle"),
        ("qq{test}", "QuoteDouble"),
        ("qw{a b}", "QuoteWords"),
        ("qx{ls}", "QuoteCommand"),
        ("m/pat/", "RegexMatch"),
        ("s/from/to/", "Substitution"),
        ("tr/a-z/A-Z/", "Transliteration"),
        ("y/a-z/A-Z/", "Transliteration"),
    ];
    
    for (input, expected_type_name) in tests {
        let mut lx = PerlLexer::new(input);
        let t = lx.next_token().unwrap();
        
        // Basic type check
        let actual_type_name = match &t.token_type {
            TokenType::QuoteSingle => "QuoteSingle",
            TokenType::QuoteDouble => "QuoteDouble",
            TokenType::QuoteWords => "QuoteWords",
            TokenType::QuoteCommand => "QuoteCommand",
            TokenType::RegexMatch => "RegexMatch",
            TokenType::Substitution => "Substitution",
            TokenType::Transliteration => "Transliteration",
            _ => "Other",
        };
        
        assert_eq!(
            actual_type_name, expected_type_name,
            "Input '{input}' produced wrong token type: {:?}",
            t.token_type
        );
    }
}

#[test]
fn heredoc_start_is_not_stringliteral() {
    let mut lx = PerlLexer::new("print <<'A';\nA\n");
    
    // First token should be 'print'
    let t1 = lx.next_token().unwrap();
    assert!(matches!(t1.token_type, TokenType::Keyword(_)));
    assert_eq!(t1.text.as_ref(), "print");
    
    // Next should be HeredocStart, not StringLiteral (whitespace is consumed automatically)
    let t2 = lx.next_token().unwrap();
    assert!(
        matches!(t2.token_type, TokenType::HeredocStart),
        "Expected HeredocStart but got {:?}",
        t2.token_type
    );
}

#[test]
fn heredoc_bare_label() {
    let mut lx = PerlLexer::new("<<EOF");
    let t = lx.next_token().unwrap();
    assert!(matches!(t.token_type, TokenType::HeredocStart));
}

#[test]
fn heredoc_indented() {
    let mut lx = PerlLexer::new("<<~END");
    let t = lx.next_token().unwrap();
    assert!(matches!(t.token_type, TokenType::HeredocStart));
}

#[test]
fn sigil_brace_is_not_identifier() {
    // Test that ${, @{, %{ are split into separate tokens
    for s in ["${", "@{", "%{"] {
        let mut lx = PerlLexer::new(s);
        let a = lx.next_token().unwrap();
        let b = lx.next_token().unwrap();
        
        // First token should be the sigil
        let sigil_char = s.chars().next().unwrap();
        assert!(
            matches!(a.token_type, TokenType::Identifier(ref id) if id.as_ref() == &sigil_char.to_string()),
            "First token of '{}' should be sigil '{}', got {:?}",
            s, sigil_char, a.token_type
        );
        
        // Second token should be LeftBrace
        assert!(
            matches!(b.token_type, TokenType::LeftBrace),
            "Second token of '{}' should be LeftBrace, got {:?}",
            s, b.token_type
        );
    }
}

#[test]
fn heredoc_label_with_space_after_chevrons() {
    // Test heredoc with space after <<
    let mut lx = PerlLexer::new("<< 'A'");
    let t = lx.next_token().unwrap();
    assert!(
        matches!(t.token_type, TokenType::HeredocStart),
        "Expected HeredocStart for '<< A', got {:?}",
        t.token_type
    );
}

#[test]
fn sigil_brace_with_trailing_junk_never_panics() {
    // These patterns previously could cause issues
    for s in ["${", "${ ", "${\n", "${}", "${ x", "${\t}"] {
        let mut lx = PerlLexer::new(s);
        let mut count = 0;
        
        // Should tokenize without panic and terminate
        while lx.next_token().is_some() {
            count += 1;
            if count > 10 {
                panic!("Lexer appears to be in infinite loop for '{}'", s);
            }
        }
    }
}

#[test]
fn malformed_substitution_never_panics() {
    // Test patterns that caused underflow
    for s in ["}s{}", "}}s{{}}", "s{", "s}", "s{{}}", "}}{s{}}{{"] {
        let mut lx = PerlLexer::new(s);
        let mut count = 0;
        
        while lx.next_token().is_some() {
            count += 1;
            assert!(count < 100, "Possible infinite loop in '{}'", s);
        }
    }
}