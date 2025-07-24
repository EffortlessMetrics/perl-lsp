use perl_parser::Parser;
use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let test_cases = vec![
        "package Foo;",
        "package Foo 1.23;",
        "package Foo::Bar;",
        "package Foo::Bar 1.23;",
        "package Foo::Bar v1.2.3;",
        "v1.2.3",  // Just the v-string
        "use v5.10;",
    ];
    
    for code in test_cases {
        println!("Testing: {}", code);
        println!("Lexer tokens:");
        
        let mut lexer = PerlLexer::new(code);
        loop {
            match lexer.next_token() {
                Some(token) => {
                    println!("  {:?}", token);
                    if matches!(token.token_type, TokenType::EOF) {
                        break;
                    }
                }
                None => break,
            }
        }
        
        println!("Parser result:");
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => println!("  ✅ Success!\n  AST: {:?}", ast.to_sexp()),
            Err(e) => println!("  ❌ Error: {}", e),
        }
        println!();
    }
}