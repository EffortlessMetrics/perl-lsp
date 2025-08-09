//! Debug v5.36 features
// use perl_parser::Parser;
use perl_lexer::PerlLexer;

fn main() {
    let input = r#"use v5.36;
use strict;
use warnings;

try {
    risky_operation();
} catch ($e) {
    warn "Error: $e";
}

defer {
    cleanup();
}

class Point {
    field $x;
    field $y;
    
    method new($x, $y) {
        $self->{x} = $x;
        $self->{y} = $y;
        return $self;
    }
}"#;

    println!("=== Input length: {} ===", input.len());
    println!("Character at position 128: {:?}", input.chars().nth(128));
    
    // Show context around position 128
    let start = 120.max(0);
    let end = 140.min(input.len());
    println!("\nContext around position 128:");
    println!("{}", &input[start..end]);
    println!("{}^", " ".repeat(128 - start));
    
    // Lexer output
    println!("\nLexer output around position 128:");
    let mut lexer = PerlLexer::new(input);
    while let Some(token) = lexer.next_token() {
        if token.start <= 128 && token.end >= 128 {
            println!("  Token at position 128: {:?}", token);
        }
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
}