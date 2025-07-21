//! Debug package parsing
use perl_parser::{Parser, TokenStream, TokenKind};
use perl_lexer::PerlLexer;

fn main() {
    let code = "package Test::Module;";
    println!("Code: {}", code);
    
    // Show lexer output
    println!("\nLexer tokens:");
    let mut lexer = PerlLexer::new(code);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
    }
    
    // Show parser tokens
    println!("\nParser tokens:");
    let mut stream = TokenStream::new(code);
    loop {
        match stream.next() {
            Ok(token) => {
                println!("  {:?}", token);
                if token.kind == TokenKind::Eof {
                    break;
                }
            }
            Err(e) => {
                println!("  Error: {}", e);
                break;
            }
        }
    }
    
    // Try parsing
    println!("\nParsing result:");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("✅ Success!");
            println!("S-expression: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}