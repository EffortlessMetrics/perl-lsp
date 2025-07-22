//! Debug version string tokenization
use perl_parser::token_stream::TokenStream;

fn main() {
    let tests = vec![
        "use v5.36",
        "use 5.036",
    ];
    
    for test in tests {
        println!("Input: {:?}", test);
        let mut stream = TokenStream::new(test);
        print!("  Tokens: ");
        while let Ok(token) = stream.next() {
            if matches!(token.kind, perl_parser::token_stream::TokenKind::Eof) {
                println!("EOF");
                break;
            }
            print!("{:?}={:?} ", token.kind, token.text);
        }
    }
}