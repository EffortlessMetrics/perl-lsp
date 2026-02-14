//! Test reference operator
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_reference_operator_basic() {
    let cases = vec![
        ("\\$scalar", "reference to scalar"),
        ("\\@array", "reference to array"),
        ("\\%hash", "reference to hash"),
        ("\\&mysub", "reference to subroutine"),
        ("\\*glob", "reference to typeglob"),
    ];

    for (input, desc) in cases {
        println!("\n=== Testing {} ===", desc);
        let mut lexer = PerlLexer::new(input);

        // Should get backslash as operator
        let token1 = lexer.next_token().unwrap();
        println!("Token 1: {:?}", token1);
        assert!(matches!(token1.token_type, TokenType::Operator(_)));
        assert_eq!(token1.text.as_ref(), "\\");

        // Should get the variable or operator
        let token2 = lexer.next_token().unwrap();
        println!("Token 2: {:?}", token2);

        // For &sub, it's tokenized as & operator
        if input.contains("&") {
            assert!(matches!(token2.token_type, TokenType::Operator(_)));
            // Get the subroutine name
            let token3 = lexer.next_token().unwrap();
            println!("Token 3: {:?}", token3);
            assert!(matches!(token3.token_type, TokenType::Identifier(_)));
        } else {
            assert!(matches!(token2.token_type, TokenType::Identifier(_)));
        }
    }
}

#[test]
fn test_typeglob_slots_fixed() {
    let input = "*foo{SCALAR} = \\$x;";
    let mut lexer = PerlLexer::new(input);

    println!("\n=== Typeglob slot syntax (fixed) ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if let TokenType::Error(msg) = &token.token_type {
            must(Err::<(), _>(format!("Unexpected error: {}", msg)));
        }
    }
}

#[test]
fn test_operator_overload_fixed() {
    let input = r#"use overload '+' => \&add;"#;
    let mut lexer = PerlLexer::new(input);

    println!("\n=== Operator overloading (fixed) ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if let TokenType::Error(msg) = &token.token_type {
            must(Err::<(), _>(format!("Unexpected error: {}", msg)));
        }
    }
}

#[test]
fn test_reference_in_expressions() {
    let cases = vec![
        ("my $ref = \\$var;", "assignment"),
        ("push @array, \\$item;", "in function call"),
        ("\\$hash{key}", "hash element reference"),
        ("\\@{$arrayref}", "reference to dereference"),
    ];

    for (input, desc) in cases {
        println!("\n=== Testing reference in {} ===", desc);
        let mut lexer = PerlLexer::new(input);
        let mut has_backslash = false;

        while let Some(token) = lexer.next_token() {
            if token.text.as_ref() == "\\" {
                has_backslash = true;
            }
            println!("Token: {:?}", token);
        }

        assert!(has_backslash, "Should have found backslash in: {}", input);
    }
}
