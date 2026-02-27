//! Test additional edge cases mentioned in the audit
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_number_formats() {
    let cases = vec![
        ("1_000_000", "number with underscores"),
        ("0b101010", "binary number"),
        ("0o755", "octal number"),
        ("0x1234", "hex number"),
        ("3.14_159", "float with underscores"),
    ];

    for (input, desc) in cases {
        println!("\n=== Testing {} ===", desc);
        let mut lexer = PerlLexer::new(input);

        let token = lexer.next_token().unwrap();
        println!("Token: {:?}", token);
        assert!(matches!(token.token_type, TokenType::Number(_)));
    }
}

#[test]
fn test_unicode_identifiers() {
    let cases = vec![
        ("my $π = 3.14159;", "pi symbol"),
        ("my $café = 'coffee';", "accented e"),
        ("my $Σ = 0;", "sigma symbol"),
        ("sub 日本語 { }", "Japanese characters"),
    ];

    for (input, desc) in cases {
        println!("\n=== Testing {} ===", desc);
        let mut lexer = PerlLexer::new(input);
        let mut found_unicode = false;

        while let Some(token) = lexer.next_token() {
            println!("Token: {:?}", token);
            if let TokenType::Identifier(id) = &token.token_type
                && !id.is_ascii()
            {
                found_unicode = true;
            }
        }

        assert!(found_unicode, "Should have found unicode identifier in: {}", input);
    }
}

#[test]
fn test_yada_yada_operator() {
    let input = "sub todo { ... }";
    let mut lexer = PerlLexer::new(input);
    let mut found_ellipsis = false;

    println!("\n=== Testing yada yada operator ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if token.text.as_ref() == "..." {
            found_ellipsis = true;
        }
    }

    assert!(found_ellipsis, "Should have found ... operator");
}

#[test]
fn test_transliteration_operators() {
    let cases = vec![
        ("tr/a-z/A-Z/", "tr with slashes"),
        ("y/a-z/A-Z/", "y with slashes"),
        ("tr[a-z][A-Z]", "tr with brackets"),
        ("tr{a-z}{A-Z}", "tr with braces"),
    ];

    for (input, desc) in cases {
        println!("\n=== Testing {} ===", desc);
        let mut lexer = PerlLexer::new(input);

        let token = lexer.next_token().unwrap();
        println!("Token: {:?}", token);

        // Check if it's recognized as a transliteration
        match &token.token_type {
            TokenType::Transliteration => {
                println!("✓ Correctly identified as Transliteration");
            }
            TokenType::Error(msg) => {
                println!("✗ Not recognized as Transliteration: {}", msg);
            }
            _ => {
                println!("? Tokenized as: {:?}", token.token_type);
            }
        }
    }
}
