use perl_parser::Parser;

fn main() {
    // Test if 'sub' keyword can be parsed as an expression
    let test = "sub";
    println!("Testing: {}", test);
    
    let mut parser = Parser::new(test);
    // Try to parse just the expression
    match parser.parse() {
        Ok(ast) => println!("Success: {:?}", ast),
        Err(e) => println!("Error: {}", e),
    }
}