//! Debug substitution tokenization
use perl_parser::{TokenStream, TokenKind};

fn main() {
    let tests = vec![
        "s/old/new/",
        "s/old/new/g",
        "tr/a-z/A-Z/",
        "y/a-z/A-Z/",
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