//! Debug compound assignment tokenization
use perl_parser::{TokenKind, TokenStream};

fn main() {
    let tests = vec!["$x += 5", "$x .= 'str'", "$x ||= 1"];

    for code in tests {
        println!("\nCode: {}", code);
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
}
