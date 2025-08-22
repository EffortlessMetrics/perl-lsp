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