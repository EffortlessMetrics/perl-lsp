//! Test v5.36 specific features
use perl_parser::Parser;

fn main() {
    // The exact test case from corpus
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

    println!("=== Testing complete v5.36 example ===");
    let mut parser = Parser::new(input);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Success! S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            println!("Error details: {:?}", e);
        }
    }
}