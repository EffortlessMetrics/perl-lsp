//! Debug ternary operator tokenization
use perl_parser::{TokenStream, TokenKind};

fn main() {
    let code = "$x ? $y : $z";
    println!("Code: {}", code);
    println!("Parser tokens:");
    let mut stream = TokenStream::new(code);
    loop {
        match stream.next() {
            Ok(token) => {
                println!("  Token: {:?}", token);
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