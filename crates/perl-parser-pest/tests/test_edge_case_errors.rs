//! Check specific edge case errors
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn check_typeglob_error() {
    let input = "*foo{SCALAR} = \\$x;";
    let mut lexer = PerlLexer::new(input);

    println!("\n=== Typeglob slot syntax ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if let TokenType::Error(msg) = &token.token_type {
            println!("ERROR: {}", msg);
        }
    }
}

#[test]
fn check_overload_error() {
    let input = r#"use overload '+' => \&add;"#;
    let mut lexer = PerlLexer::new(input);

    println!("\n=== Operator overloading ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if let TokenType::Error(msg) = &token.token_type {
            println!("ERROR: {}", msg);
        }
    }
}
