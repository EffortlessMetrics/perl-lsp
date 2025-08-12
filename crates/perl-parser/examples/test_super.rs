//! Test SUPER method calls
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Method calls
        ("$self->method()", "simple method call"),
        ("$self->SUPER::method()", "SUPER method call"),
        ("$obj->Some::Package::method()", "qualified method call"),
        // Direct calls
        ("SUPER::method()", "direct SUPER call"),
        ("Package::method()", "direct package call"),
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
