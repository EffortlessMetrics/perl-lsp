use perl_parser::Parser;
use perl_lexer::{PerlLexer, TokenType};

fn main() {
    let code = "<<>>";
    println!("Debugging: {}\n", code);
    
    // First, let's see what tokens the lexer produces
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
    
    // Now let's try parsing
    println!("\nParser result:");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => println!("Success! AST: {:#?}", ast),
        Err(e) => println!("Error: {}", e),
    }
    
    // Also test in context
    println!("\n\nIn context: while (<<>>) {{}}");
    let mut parser = Parser::new("while (<<>>) { }");
    match parser.parse() {
        Ok(ast) => println!("Success! AST: {:#?}", ast),
        Err(e) => println!("Error: {}", e),
    }
}