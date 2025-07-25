use perl_parser::{Parser, TokenStream, TokenKind};

fn main() {
    let code = "my $♥ = 'love';";
    println!("Testing: {}", code);
    
    // First check what tokens we get
    println!("\nTokens from TokenStream:");
    let mut stream = TokenStream::new(code);
    let mut count = 0;
    while !stream.is_eof() && count < 10 {
        match stream.peek() {
            Ok(token) => {
                println!("  {:?} '{}' at {}", token.kind, token.text, token.start);
                stream.next().unwrap();
            }
            Err(e) => {
                println!("  Error: {:?}", e);
                break;
            }
        }
        count += 1;
    }
    
    // Now try parsing
    println!("\nParsing:");
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            println!("Success! AST:\n{}", ast.to_sexp());
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}