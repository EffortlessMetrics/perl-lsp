use perl_lexer::{PerlLexer, TokenType};
use perl_parser::Parser;

fn main() {
    let code = "sub foo ($$$) { }";
    println!("Debugging: {}\n", code);

    // First, let's see what tokens the lexer produces
    println!("Lexer tokens:");
    let mut lexer = PerlLexer::new(code);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }

    // Now let's try parsing
    println!("\nParser result:");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => println!("Success! AST: {:#?}", ast),
        Err(e) => println!("Error: {}", e),
    }
}
