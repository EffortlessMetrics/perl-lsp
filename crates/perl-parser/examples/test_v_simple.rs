//! Test just version strings
use perl_parser::Parser;

fn main() {
    let tests = vec!["use v5.36", "use 5.036"];

    for test in tests {
        println!("Testing: {}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => println!("  ✅ S-expr: {}", ast.to_sexp()),
            Err(e) => println!("  ❌ Error: {}", e),
        }
    }
}
