//! Test power operator specifically
use perl_parser::Parser;
use perl_lexer::PerlLexer;

fn main() {
    let input = "2 ** 3";
    println!("=== Testing: {} ===", input);
    
    // First check lexer output
    println!("\nLexer output:");
    let mut lexer = PerlLexer::new(input);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
    
    // Then try parser
    println!("\nParser output:");
    let mut parser = Parser::new(input);
    match parser.parse() {
        Ok(ast) => {
            println!("  Success! AST: {:?}", ast);
            println!("  S-expr: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
}