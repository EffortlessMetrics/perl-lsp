//! Debug no keyword
use perl_parser::{TokenKind, TokenStream};

fn main() {
    let code = "no strict;";
    println!("Code: {}", code);
    println!("\nParser tokens:");

    let mut stream = TokenStream::new(code);
    loop {
        match stream.next() {
            Ok(token) => {
                println!(
                    "  Token: {:?} '{}' (kind={:?})",
                    token, token.text, token.kind
                );
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
