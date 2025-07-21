//! Debug bracket tokenization
use perl_parser::{TokenStream, TokenKind};

fn main() {
    let code = "$array[0]";
    println!("Code: {}", code);
    println!("Parser tokens:");
    
    let mut stream = TokenStream::new(code);
    loop {
        match stream.next() {
            Ok(token) => {
                println!("  Token: {:?} '{}' (kind={:?})", token, token.text, token.kind);
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