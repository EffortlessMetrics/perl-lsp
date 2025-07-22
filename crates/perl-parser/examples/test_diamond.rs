//! Test diamond operator edge cases
use perl_parser::Parser;

fn main() {
    let tests = vec![
        "while (<>) { print }",
        "while (<>) { print; }",
        "while (<STDIN>) { print }",
        "while (<$fh>) { print }",
        "for (<>) { print }",
        "if (<>) { print }",
        "print while <>",
        "print for <>",
    ];
    
    for test in tests {
        println!("Testing: {:?}", test);
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => println!("  ✅ S-expr: {}", ast.to_sexp()),
            Err(e) => println!("  ❌ Error: {}", e),
        }
    }
}