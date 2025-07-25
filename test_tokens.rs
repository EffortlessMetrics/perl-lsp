use perl_parser::{Parser, TokenStream};

fn main() {
    let code = "@{ [ 1, 2, 3 ] }";
    println!("Code: {}", code);
    println!("\nTokens:");
    
    let mut tokens = TokenStream::new(code);
    while let Ok(token) = tokens.next() {
        if matches!(token.kind, perl_parser::TokenKind::Eof) {
            break;
        }
        println!("  {:?} => {:?} at {}", token.kind, token.text, token.start);
    }
}