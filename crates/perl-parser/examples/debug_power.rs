//! Debug power operator tokenization
use perl_parser::{TokenStream, TokenKind};

fn main() {
    let tests = vec![
        "$x **= 2",
        "$x ** 2",
    ];
    
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