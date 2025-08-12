//! Debug division operator parsing issue

use perl_parser::Parser;

fn main() {
    let test_cases = vec![
        "$a + $b", // Works
        "$a - $b", // Works
        "$a * $b", // Works
        "$a / $b", // Fails
        "10 / 2",  // Check literal division
        "$x/$y",   // No spaces
        "$a / 2",  // Variable / literal
        "2 / $b",  // Literal / variable
    ];

    for code in test_cases {
        println!("\nTesting: {}", code);
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("  ✅ Success! AST: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("  ❌ Failed: {:?}", e);
            }
        }
    }
}
