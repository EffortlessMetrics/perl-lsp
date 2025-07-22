//! Debug exit parsing
use perl_parser::token_stream::{TokenStream, TokenKind};
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "exit",
        "print",
    ];

    for test in tests {
        println!("\nInput: {:?}", test);
        
        // Debug tokens
        let mut stream = TokenStream::new(test);
        print!("  Tokens: ");
        while let Ok(token) = stream.next() {
            if matches!(token.kind, TokenKind::Eof) {
                println!("EOF");
                break;
            }
            print!("{:?}={:?} ", token.kind, token.text);
        }
        
        // Try parsing
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("  Parse: ✅ {}", ast.to_sexp());
            }
            Err(e) => {
                println!("  Parse: ❌ {}", e);
            }
        }
    }
}