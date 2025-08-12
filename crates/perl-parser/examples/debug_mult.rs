//! Debug multiplication parsing

use perl_lexer::PerlLexer;
use perl_parser::Parser;

fn main() {
    let code = "$result = ($a + $b) * $c;";
    println!("Code: {}", code);

    // Show lexer output
    println!("\nLexer tokens:");
    let mut lexer = PerlLexer::new(code);
    while let Some(token) = lexer.next_token() {
        println!("  {:?}", token);
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }

    // Show what parser sees via token stream
    println!("\nToken stream:");
    use perl_parser::token_stream::{TokenKind, TokenStream};
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
}
