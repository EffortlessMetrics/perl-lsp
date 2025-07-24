//! Test exact failing anonymous subroutine case
use perl_parser::Parser;
use perl_lexer::PerlLexer;

fn main() {
    let input = r#"my $anon = sub { return "anonymous"; };"#;
    println!("=== Testing: {} ===", input);
    println!("Length: {}", input.len());
    
    // First check lexer output with positions
    println!("\nLexer output:");
    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        println!("  {:?} at positions {}-{}", token.token_type, token.start, token.end);
        tokens.push(token.clone());
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }
    
    // Show the specific tokens around position 35
    println!("\nTokens around position 35:");
    for (i, token) in tokens.iter().enumerate() {
        if token.start <= 35 && token.end >= 35 {
            println!("  Token {}: {:?} '{}' at {}-{}", i, token.token_type, token.text, token.start, token.end);
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