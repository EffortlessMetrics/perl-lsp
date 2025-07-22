use perl_parser::{Parser, TokenKind};

fn main() {
    let input = "$hash->{key}->[0]->{sub}";
    println!("Input: {}", input);
    
    // Parse
    println!("\nParsing:");
    let mut parser = Parser::new(input);
    match parser.parse() {
        Ok(ast) => println!("Success: {:?}", ast),
        Err(e) => println!("Error: {}", e),
    }
}