//! Debug tokenization
use perl_parser::token_stream::{TokenKind, TokenStream};

fn main() {
    let tests = vec!["2 ** 3", "$x ** $y", "**"];

    for test in tests {
        println!("\nInput: {:?}", test);
        let mut stream = TokenStream::new(test);

        while let Ok(token) = stream.next() {
            if matches!(token.kind, TokenKind::Eof) {
                break;
            }
            println!("  Token: {:?} = {:?}", token.kind, token.text);
        }
    }
}
