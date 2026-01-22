use perl_parser::Parser;

fn main() {
    // Test with exactly 257 nested braces (should fail)
    let mut code = String::new();
    for _ in 0..257 {
        code.push_str("{ ");
    }
    for _ in 0..257 {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    match parser.parse() {
        Ok(_) => println!("UNEXPECTED: Parsing succeeded for 257 levels"),
        Err(e) => println!("Expected: Parsing failed with: {}", e),
    }

    // Test with 256 nested braces (should succeed)
    let mut code2 = String::new();
    for _ in 0..256 {
        code2.push_str("{ ");
    }
    for _ in 0..256 {
        code2.push_str("} ");
    }

    let mut parser2 = Parser::new(&code2);
    match parser2.parse() {
        Ok(_) => println!("Expected: Parsing succeeded for 256 levels"),
        Err(e) => println!("UNEXPECTED: Parsing failed for 256 levels with: {}", e),
    }
}
