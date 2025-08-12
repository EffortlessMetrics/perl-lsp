//! Test complex expression parsing

use perl_parser::Parser;

fn main() {
    // Test various expressions
    let test_cases = vec![
        "$result = $a + $b;",
        "$result = ($a + $b);",
        "$result = ($a + $b) * $c;",
        "$x = 1 * 2 + 3;",
    ];

    for code in test_cases {
        println!("\n=== Parsing: {} ===", code);

        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(ast) => {
                println!("AST: {:?}", ast);
                println!("S-expression: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
