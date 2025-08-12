//! Debug special variable tokenization
use perl_parser::{TokenKind, TokenStream};

fn main() {
    let tests = vec!["$_", "$!", "$$", "$@", "$?"];

    for code in tests {
        println!("\nCode: {}", code);
        println!("Parser tokens:");

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
}
