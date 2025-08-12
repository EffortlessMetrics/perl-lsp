//! Test return statement semicolons
use perl_parser::Parser;

fn main() {
    let tests = vec![
        ("return 42", "return without semicolon"),
        ("return 42;", "return with semicolon"),
        ("{ return 42 }", "block with return without semicolon"),
        ("{ return 42; }", "block with return with semicolon"),
        ("sub { return 42 }", "sub with return without semicolon"),
        ("sub { return 42; }", "sub with return with semicolon"),
    ];

    for (test, desc) in tests {
        println!("\n=== Testing: {} ===", desc);
        println!("Input: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success! S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
